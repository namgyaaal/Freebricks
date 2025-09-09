use std::{borrow::Cow, ops::DerefMut};

/*

    Not officially a tree but soon to be!

*/
use crate::{
    common::asset_cache::AssetCache,
    ecs::parts::*,
    render::{
        camera::*,
        details::part_details::{part_details_get, part_details_init},
        part_queue::PartQueue,
        parts::*,
        render_state::{FrameInfo, RenderState},
        scene_map::{SceneMap, SpatialKey},
        texture::*,
    },
};
use bevy_ecs::prelude::*;
use glam::Vec3;
use wgpu::{DynamicOffset, util::DeviceExt};

#[derive(Resource)]
pub struct SceneTree {
    pub instancing_bind_group: wgpu::BindGroup,
    pub uniform_bind_groups: Vec<wgpu::BindGroup>,
    pub removal_buffer: Vec<Entity>,
    pub map: SceneMap<32>,
    pub part_queue: PartQueue,
}

impl SceneTree {
    /// Called at beginning of game.
    /// Requires renderstate to be initialized and must be called before gen_bricks
    pub fn init(world: &mut World) -> Result<()> {
        let render_state = world
            .get_resource::<RenderState>()
            .expect("SceneTree::init(), expected Render State");

        let asset_cache = world
            .get_resource::<AssetCache>()
            .expect("SceneTree::init(), expected Asset Cache");

        let brick_diffuse = asset_cache
            .get_image("textures/studs.png")
            .expect("Couldn't get brick texture");

        let shader_source = asset_cache
            .get_shader("shaders/bricks.wgsl")
            .expect("Couldn't get brick texture");

        let device = &render_state.device;
        let queue = &render_state.queue;
        let config = &render_state.config;
        part_buffer_init(device);
        part_details_init(device, config, Cow::from(shader_source));
        /*
           Brick Texture Layout
        */
        let part_layout = &part_details_get().part_bind_layout;
        let brick_texture = Texture::create_brick_texture(&brick_diffuse, device, queue);

        let dir = glam::Vec3::new(-0.5, -0.7, -1.0).normalize();
        let light_direction = [dir.x, dir.y, dir.z, 0.0];
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Buffer"),
            contents: bytemuck::cast_slice(&[light_direction]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let camera_buffer = &world.get_resource::<Camera>().unwrap().buffer;

        let part_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Brick Texture Group"),
            layout: &part_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&brick_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&brick_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: light_buffer.as_entire_binding(),
                },
            ],
        });

        /*
        let mut binding = camera_buffer.as_entire_buffer_binding();
        binding.offset = 0;

        wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(binding),
        };
        */

        let pq = PartQueue::new(device);

        world.add_observer(Self::handle_part_removal);
        world.insert_resource(Self {
            instancing_bind_group: part_group,
            uniform_bind_groups: Vec::new(),
            removal_buffer: Vec::new(),
            map: SceneMap::new(),
            part_queue: pq,
        });

        Ok(())
    }

    pub fn handle_part_removal(trigger: Trigger<OnRemove, Part>, mut st: ResMut<SceneTree>) {
        let entity = trigger.target();
        st.removal_buffer.push(entity);
    }

    /// Adjust possible instance and uniform buffers on event of objects being deleted
    pub fn remove_bricks(mut st: ResMut<SceneTree>) -> Result<()> {
        let scene_tree = st.deref_mut();
        if scene_tree.removal_buffer.is_empty() {
            return Ok(());
        }

        let entities: Vec<Entity> = scene_tree.removal_buffer.drain(..).collect();
        for entity in entities {
            scene_tree.map.remove(entity)?;
        }
        Ok(())
    }

    /// Called in update loop if bricks are added to the scene during the game
    /// Reorders the BufferIndex component to its position in the instance buffer
    pub fn add_bricks(
        _state: Res<RenderState>,
        mut st: ResMut<SceneTree>,
        query: Query<QPartRenderUpdate, FPartAdd>,
    ) -> Result<()> {
        //let queue = &state.queue;

        for brick in query.iter() {
            // Give buffer index the size of the vector for now until we need multiple buffers
            let uniform = Part::to_instance(
                brick.part,
                brick.studs,
                brick.position,
                brick.rotation,
                brick.size,
                brick.color,
            );
            if let Err(e) = st
                .map
                .set(brick.entity, *brick.part, brick.position.0, uniform)
            {
                println!("{:?}", e);
            };
        }

        Ok(())
    }

    pub fn update_bricks(
        scene: Res<RenderState>,
        mut st: ResMut<SceneTree>,
        query: Query<QPart, FPartChangeTransform>,
    ) -> Result<()> {
        let st = st.deref_mut();
        let scene = &scene;
        #[allow(unused)]
        let device = &scene.device;
        #[allow(unused)]
        let queue = &scene.queue;

        for brick in query.iter() {
            let uniform = Part::to_instance(
                brick.part,
                brick.studs,
                brick.position,
                brick.rotation,
                brick.size,
                brick.color,
            );

            st.map
                .set(brick.entity, *brick.part, brick.position.0, uniform)?;
        }

        Ok(())
    }

    pub fn write_buffers(
        state: Res<RenderState>,
        mut scene_tree: ResMut<SceneTree>,
        mut info: ResMut<FrameInfo>,
        camera: Res<Camera>,
    ) {
        let scene_tree = scene_tree.deref_mut();
        let device = &state.device;
        let encoder = info
            .command
            .as_mut()
            .expect("write_buffers() expects command encoder");

        let cell_size = scene_tree.map.get_size() as f32;
        let mut keys: Vec<&SpatialKey> = scene_tree
            .map
            .spatial_map
            .keys()
            .into_iter()
            .filter(|key| {
                let center = key.position.as_vec3() + (cell_size / 2.0);
                let extent = Vec3::new(cell_size, cell_size, cell_size) / 0.60;
                camera.check_bounds(center, extent)
            })
            .collect();
        keys.sort_by(|a, b| {
            a.position
                .as_vec3()
                .distance(camera.position)
                .partial_cmp(&b.position.as_vec3().distance(camera.position))
                .expect("No idea how cmp can fail but it did")
        });

        // TODO: Find out better way to handle this
        let mut uniforms: Vec<PartInstance> = Vec::new();
        for key in keys {
            let Some(cell) = scene_tree.map.spatial_map.get(key) else {
                continue;
            };
            uniforms.extend_from_slice(&cell.buffers);
        }

        scene_tree
            .part_queue
            .map_slice(device, encoder, Part::Brick, &uniforms);
        scene_tree.part_queue.submit();
    }

    pub fn render(
        mut scene_tree: ResMut<SceneTree>,
        _scene: Res<RenderState>,
        mut info: ResMut<FrameInfo>,
    ) {
        let pass = info
            .pass
            .as_mut()
            .expect("SceneTree::render(), expected RenderPass");
        pass.set_pipeline(&part_details_get().instancing_pipeline);
        pass.set_bind_group(0, &scene_tree.instancing_bind_group, &[]);
        //pass.set_bind_group(1, &scene_tree.scene_bg, &[]);

        scene_tree.part_queue.drain_instances(|ready| {
            let (vb, ib) = part_buffer_fetch(ready.part_type);
            pass.set_vertex_buffer(0, vb.slice(..));
            pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint16);

            pass.set_vertex_buffer(1, ready.buffer.slice(..));
            pass.draw_indexed(0..BRICK_INDICES.len() as _, 0, 0..ready.len as _);
        });

        pass.set_pipeline(&part_details_get().uniform_pipeline);
        pass.set_bind_group(0, &scene_tree.instancing_bind_group, &[]);

        scene_tree.part_queue.drain_uniforms(|ready| {
            let (vb, ib) = part_buffer_fetch(ready.part_type);
            let (bind_group, offsets) = ready
                .bind_and_offsets
                .as_ref()
                .expect("Should have bind and offsets");

            pass.set_vertex_buffer(0, vb.slice(..));
            pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint16);

            for offset in offsets {
                pass.set_bind_group(1, bind_group, &[*offset]);
                pass.draw_indexed(0..BRICK_INDICES.len() as _, 0, 0..1);
            }
        });
    }

    pub fn recall(mut scene_tree: ResMut<SceneTree>) {
        scene_tree.part_queue.recall();
    }
}
