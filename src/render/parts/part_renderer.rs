use std::{borrow::Cow, ops::DerefMut};

/*

    Not officially a tree but soon to be!

*/
use crate::{
    common::asset_cache::AssetCache,
    ecs::parts::*,
    render::{
        camera::*,
        parts::{
            part_buffers::PartBuffers, part_details::PartDetails, part_formats::PartInstance,
            part_queue::DefaultPartQueue,
        },
        render_state::{FrameInfo, RenderState},
        spatial_map::{DefaultSpatialKey, DefaultSpatialMap, SpatialMap},
        texture::*,
    },
};
use bevy_ecs::prelude::*;
use bevy_platform::collections::HashMap;
use glam::{Affine3A, Vec3, Vec3A};
use wgpu::util::DeviceExt;

#[derive(Resource)]
pub struct PartRenderer {
    pub instancing_bind_group: wgpu::BindGroup,
    pub uniform_bind_groups: Vec<wgpu::BindGroup>,
    pub removal_buffer: Vec<Entity>,
    pub map: DefaultSpatialMap,
    // Part type and opacity
    pub part_queue: DefaultPartQueue,
}

impl PartRenderer {
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

        PartBuffers::init(device);
        PartDetails::init(device, config, Cow::from(shader_source));
        /*
           Brick Texture Layout
        */
        let part_layout = &PartDetails::get().part_bind_layout;
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

        let pq = DefaultPartQueue::new(device);

        world.add_observer(Self::handle_part_removal);
        world.insert_resource(Self {
            instancing_bind_group: part_group,
            uniform_bind_groups: Vec::new(),
            removal_buffer: Vec::new(),
            map: SpatialMap::new(),
            part_queue: pq,
        });

        Ok(())
    }

    pub fn handle_part_removal(trigger: Trigger<OnRemove, Part>, mut st: ResMut<PartRenderer>) {
        let entity = trigger.target();
        st.removal_buffer.push(entity);
    }

    /// Adjust possible instance and uniform buffers on event of objects being deleted
    pub fn remove_bricks(mut st: ResMut<PartRenderer>) -> Result<()> {
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
        mut st: ResMut<PartRenderer>,
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
        mut st: ResMut<PartRenderer>,
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
        mut scene_tree: ResMut<PartRenderer>,
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

        // Get keys that are in bounds of camera
        let mut keys: Vec<&DefaultSpatialKey> = scene_tree
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
        // Sort keys by distance of camera
        keys.sort_by(|a, b| {
            a.position
                .as_vec3()
                .distance(camera.position)
                .partial_cmp(&b.position.as_vec3().distance(camera.position))
                .expect("No idea how cmp can fail but it did")
        });
        // Separate keys into transparent and opaque buckets
        let (opaque, transparent): (Vec<&DefaultSpatialKey>, Vec<&DefaultSpatialKey>) =
            keys.iter().partition(|k| k.opaque);

        let mut cells: HashMap<(Part, bool), Vec<PartInstance>> = HashMap::new();
        // Collect opaque cells
        for key in opaque {
            let Some(cell) = scene_tree.map.spatial_map.get(key) else {
                continue;
            };
            let vec = cells.entry((key.part, key.opaque)).or_insert(Vec::new());
            vec.extend_from_slice(&cell.buffers);
        }
        // Collect transparent cells (also do extra sorting)
        for key in transparent {
            let Some(cell) = scene_tree.map.spatial_map.get(key) else {
                continue;
            };
            let mut new_vec = cell.buffers.clone();
            new_vec.sort_by(|a, b| {
                let pos_a: Vec3A = Affine3A::from_cols_array_2d(&a.model).translation;
                let pos_b: Vec3A = Affine3A::from_cols_array_2d(&b.model).translation;

                pos_a
                    .distance(camera.position.into())
                    .partial_cmp(&pos_b.distance(camera.position.into()))
                    .expect("huh")
                    .reverse()
            });
            let vec = cells.entry((key.part, key.opaque)).or_insert(Vec::new());
            vec.extend_from_slice(&new_vec);
        }

        for (key, value) in cells {
            scene_tree
                .part_queue
                .map_slice(device, encoder, key, &value);
        }

        scene_tree.part_queue.finish();
    }

    pub fn render(
        mut scene_tree: ResMut<PartRenderer>,
        _scene: Res<RenderState>,
        mut info: ResMut<FrameInfo>,
    ) {
        let scene_tree = scene_tree.deref_mut();
        let pass = info
            .pass
            .as_mut()
            .expect("SceneTree::render(), expected RenderPass");
        pass.set_pipeline(&PartDetails::get().get_pipeline(false, true));
        pass.set_bind_group(0, &scene_tree.instancing_bind_group, &[]);
        //pass.set_bind_group(1, &scene_tree.scene_bg, &[]);

        scene_tree.part_queue.drain_instances(|ready| {
            let (vb, ib) = PartBuffers::fetch(ready.key.0);
            pass.set_vertex_buffer(0, vb.slice(..));
            pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint16);

            pass.set_vertex_buffer(1, ready.buffer.slice(..));
            pass.draw_indexed(
                0..PartBuffers::get_index_count(ready.key.0) as _,
                0,
                0..ready.len as _,
            );
        });

        scene_tree.part_queue.drain_uniforms(|ready| {
            if ready.key.1 {
                pass.set_pipeline(PartDetails::get().get_pipeline(true, true));
            } else {
                pass.set_pipeline(PartDetails::get().get_pipeline(true, false));
            }
            pass.set_bind_group(0, &scene_tree.instancing_bind_group, &[]);

            let (vb, ib) = PartBuffers::fetch(ready.key.0);
            let (bind_group, offsets) = ready
                .bind_and_offsets
                .as_ref()
                .expect("Should have bind and offsets");

            pass.set_vertex_buffer(0, vb.slice(..));
            pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint16);

            for offset in offsets {
                pass.set_bind_group(1, bind_group, &[*offset]);
                pass.draw_indexed(0..PartBuffers::get_index_count(ready.key.0) as _, 0, 0..1);
            }
        });
    }

    pub fn recall(mut scene_tree: ResMut<PartRenderer>) {
        scene_tree.part_queue.recall();
    }
}
