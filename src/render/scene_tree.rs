use std::{borrow::Cow, collections::VecDeque, num::NonZero, ops::DerefMut, u16};

/*

    Not officially a tree but soon to be!

*/
use crate::{
    common::asset_cache::AssetCache,
    ecs::{
        common::{Color, Position, Rotation, Size},
        parts::*,
        render::BufferIndex,
    },
    render::{
        camera::*,
        parts::*,
        render_state::{FrameInfo, RenderState},
        scene_map::{SceneMap, SpatialKey},
        texture::*,
    },
};
use bevy_ecs::prelude::*;
use glam::Vec3;
use rand::{Rng, SeedableRng, rngs::SmallRng, seq::IndexedRandom};
use tracing::info;
use wgpu::util::{DeviceExt, StagingBelt};

const MAX_INSTANCE_BUFFER_COUNT: usize = u16::MAX as usize * 2;
const MAX_CHUNK_SIZE: usize = MAX_INSTANCE_BUFFER_COUNT / 4;

#[derive(Resource)]
pub struct SceneTree {
    pub pipeline: wgpu::RenderPipeline,
    pub brick_vb: wgpu::Buffer,
    pub brick_ib: wgpu::Buffer,

    pub wedge_vb: wgpu::Buffer,
    pub wedge_ib: wgpu::Buffer,

    pub instance_buffer: wgpu::Buffer,
    pub brick_buffers: StagingBelt,
    pub scene_bg: wgpu::BindGroup,
    pub texture_bg: wgpu::BindGroup,
    pub bricks: Vec<PartUniform>,
    drawn_bricks: usize,
    pub clean_queue: VecDeque<u32>,
    pub map: SceneMap<32>,
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

        let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Brick Vertex Buffer"),
            contents: bytemuck::cast_slice(BRICK_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Brick Index Bufffer"),
            contents: bytemuck::cast_slice(BRICK_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let wvb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Wedge Vertex Buffer"),
            contents: bytemuck::cast_slice(WEDGE_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let wib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Wedge Index Buffer"),
            contents: bytemuck::cast_slice(WEDGE_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        /*
           Brick Texture Layout
        */

        let brick_texture = Texture::create_brick_texture(&brick_diffuse, device, queue);
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Brick Texture Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let texture_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Brick Texture Group"),
            layout: &texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&brick_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&brick_texture.sampler),
                },
            ],
        });

        /*
           Scene Kit Layout
               [Lights, Camera]--Layout and Bind Group

        */

        /*
           Quick mock-up of a light until we need one.
        */

        let dir = glam::Vec3::new(-0.5, -0.7, -1.0).normalize();
        let light_direction = [dir.x, dir.y, dir.z, 0.0];
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Buffer"),
            contents: bytemuck::cast_slice(&[light_direction]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let camera_buffer = &world.get_resource::<Camera>().unwrap().buffer;

        let scene_kit_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Scene Kit Layout"),
            entries: &[
                // Camera Entry
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Light Entry
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let scene_kit_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Scene Kit Group"),
            layout: &scene_kit_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: light_buffer.as_entire_binding(),
                },
            ],
        });

        /*
           Render Pipeline definition
        */

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("SceneTree Pipeline Layout"),
                bind_group_layouts: &[&texture_layout, &scene_kit_layout],
                push_constant_ranges: &[],
            });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Some Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::from(shader_source)),
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SceneTree Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[PartVertex::desc(), PartUniform::desc_instancing()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let bricks: Vec<PartUniform> = Vec::with_capacity(MAX_INSTANCE_BUFFER_COUNT);

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Brick Instance Buffer"),
            size: (std::mem::size_of::<PartUniform>() * MAX_INSTANCE_BUFFER_COUNT) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        world.add_observer(Self::handle_index_removal);
        world.insert_resource(Self {
            pipeline: render_pipeline,
            brick_vb: vb,
            brick_ib: ib,
            wedge_vb: wvb,
            wedge_ib: wib,
            instance_buffer: instance_buffer,
            brick_buffers: StagingBelt::new(MAX_CHUNK_SIZE as u64),
            scene_bg: scene_kit_group,
            texture_bg: texture_group,
            bricks: bricks,
            drawn_bricks: 0,
            clean_queue: VecDeque::new(),
            map: SceneMap::new(),
        });

        Ok(())
    }

    pub fn handle_index_removal(
        trigger: Trigger<OnRemove, BufferIndex>,
        indices: Query<&BufferIndex>,
        mut st: ResMut<SceneTree>,
    ) {
        let Ok(b_index) = indices.get(trigger.target()) else {
            return;
        };

        let Some(index) = b_index.0 else {
            return;
        };
        st.clean_queue.push_back(index);
    }

    /// Adjust possible instance and uniform buffers on event of objects being deleted
    pub fn remove_bricks(
        mut st: ResMut<SceneTree>,
        mut query: Query<QPartRenderUpdate>,
        //mut er: EventReader<RenderCleanup>,
    ) {
        if st.clean_queue.is_empty() {
            return;
        }

        let indices: Vec<u32> = st.clean_queue.drain(..).collect();

        let mut index: u32 = 0;
        st.bricks.retain_mut(|_| {
            let remove = !indices.contains(&index);
            index += 1;
            remove
        });

        index = 0;

        for mut bi in query.iter_mut() {
            bi.buffer_index.0 = Some(index);
            index += 1;
        }
    }

    /// Called in update loop if bricks are added to the scene during the game
    /// Reorders the BufferIndex component to its position in the instance buffer
    pub fn add_bricks(
        state: Res<RenderState>,
        mut st: ResMut<SceneTree>,
        mut query: Query<QPartRenderUpdate, FPartAdd>,
    ) -> Result<()> {
        //let queue = &state.queue;

        for mut brick in query.iter_mut() {
            // Give buffer index the size of the vector for now until we need multiple buffers
            brick.buffer_index.0 = Some(st.bricks.len() as u32);

            let uniform = Part::to_uniform(
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

            st.bricks.push(uniform);
        }
        // Again, using this until we have multiple buffers.
        //if let Some(buffer) = st.brick_ibos.first() {
        //    queue.write_buffer(buffer, 0, bytemuck::cast_slice(&st.bricks));
        //}
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
            let uniform = Part::to_uniform(
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

        // Full update of instance buffer
        //if let Some(buffer) = st.brick_ibos.first() {
        //    //queue.write_buffer(buffer, 0, bytemuck::cast_slice(&st.bricks));
        //}
    }

    pub fn write_buffers(
        state: Res<RenderState>,
        mut scene_tree: ResMut<SceneTree>,
        mut info: ResMut<FrameInfo>,
        mut i: Local<u64>,
        camera: Res<Camera>,
    ) {
        let scene_tree = scene_tree.deref_mut();
        let encoder = info
            .command
            .as_mut()
            .expect("write_buffers() expects command encoder");

        let struct_size = std::mem::size_of::<PartUniform>();
        // BufferViewMut with a small size crashes.
        let max = usize::max(scene_tree.bricks.len(), 1024);

        let belt = &mut scene_tree.brick_buffers;
        let mut view = belt.write_buffer(
            encoder,
            &scene_tree.instance_buffer,
            0,
            NonZero::new((max * struct_size) as u64).unwrap(),
            &state.device,
        );

        *i += 1;
        let mut count = 0;
        let mut batches = 0;
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
                .expect("wut")
        });

        for key in keys {
            let cell = scene_tree.map.spatial_map.get(key).expect("wut");

            let center = key.position.as_vec3() + (cell_size / 2.0);
            let extent = Vec3::new(cell_size, cell_size, cell_size) / 0.60;

            if !camera.check_bounds(center, extent) {
                continue;
            }

            if count + cell.buffers.len() > max {
                break;
            }
            let len = cell.buffers.len();

            let start = count * struct_size;
            let end = (count + len) * struct_size;

            view[start..end].copy_from_slice(bytemuck::cast_slice(&cell.buffers));

            count += cell.buffers.len();
            batches += 1;
        }
        drop(view);

        scene_tree.drawn_bricks = count;

        if *i % 40 == 0 {
            println!(
                "Drawing: {} bricks from {} batches",
                scene_tree.drawn_bricks, batches
            );
        }
        belt.finish();
    }

    pub fn render(
        mut scene_tree: ResMut<SceneTree>,
        scene: Res<RenderState>,
        mut info: ResMut<FrameInfo>,
    ) {
        let st = scene_tree.deref_mut();
        let device = &scene.device;

        let encoder = info
            .command
            .as_mut()
            .expect("SceneTree::render(), expected Command Encoder");

        let pass = info
            .pass
            .as_mut()
            .expect("SceneTree::render(), expected RenderPass");
        pass.set_pipeline(&scene_tree.pipeline);
        pass.set_bind_group(0, &scene_tree.texture_bg, &[]);
        pass.set_bind_group(1, &scene_tree.scene_bg, &[]);

        pass.set_vertex_buffer(0, scene_tree.brick_vb.slice(..));

        pass.set_index_buffer(scene_tree.brick_ib.slice(..), wgpu::IndexFormat::Uint16);

        //ueue.write_buffer(buffer, 0, bytemuck::cast_slice(&st.bricks));

        // Remove in future, just having one instance buffer for now
        //assert!(scene_tree.brick_ibos.len() == 1);
        pass.set_vertex_buffer(1, scene_tree.instance_buffer.slice(..));
        //for buf in &scene_tree.brick_ibos {
        //    pass.set_vertex_buffer(1, buf.slice(..));
        //}
        // Coupled with assert, this needs to be refactored once we "split up" scenes.

        pass.draw_indexed(0..36, 0, 0..1 as _);

        pass.set_vertex_buffer(0, scene_tree.brick_vb.slice(..));
        pass.set_index_buffer(scene_tree.brick_ib.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(
            0..BRICK_INDICES.len() as u32,
            0,
            1..(scene_tree.drawn_bricks) as _,
        );

        // (scene_tree.drawn_bricks) as _);
    }

    pub fn cleanup(mut scene_tree: ResMut<SceneTree>) {
        scene_tree.brick_buffers.recall();
    }
}
