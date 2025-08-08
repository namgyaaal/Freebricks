use std::borrow::Cow;

/*

    Not officially a tree but soon to be! 

*/
use bevy_ecs::prelude::*;
use wgpu::util::DeviceExt;
use crate::{
    common::{asset_cache::{AssetCache}},
    components::bricks::*, 
    render::{bricks::*, camera::*, render_state::{RenderPassInfo, RenderState}, texture::*}
};

#[derive(Resource)]
pub struct SceneTree {
    pub pipeline: wgpu::RenderPipeline,
    pub brick_vb: wgpu::Buffer, 
    pub brick_ib: wgpu::Buffer,
    pub brick_ibos: Vec<wgpu::Buffer>,
    pub scene_bg: wgpu::BindGroup,
    pub texture_bg: wgpu::BindGroup,
    pub bricks: Vec<BrickUniform>
}

impl SceneTree {
    pub fn init(world: &mut World) {
        let render_state = world.get_resource::<RenderState>()
            .expect("SceneTree::init(), expected Render State");

        let asset_cache = world.get_resource::<AssetCache>()
            .expect("SceneTree::init(), expected Asset Cache");

        let brick_diffuse = asset_cache
            .get_image("textures/studs.png")
            .expect("Couldn't get brick texture");

        let shader_source = asset_cache 
            .get_shader("shaders/bricks.wgsl")
            .expect("Couldn't get brick texture");


        let device = &render_state.device;
        let queue  = &render_state.queue;
        let config = &render_state.config;

        let vb = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Brick Vertex Buffer"), 
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let ib = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Brick Index BUffer"),
                contents: bytemuck::cast_slice(INDICES),
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        /*
            Brick Texture Layout 
         */

        let brick_texture = Texture::create_brick_texture(&brick_diffuse, device, queue);
        let texture_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
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
                ]
            }
        );

        let texture_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some("Brick Texture Group"),
                layout: &texture_layout, 
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0, 
                        resource: wgpu::BindingResource::TextureView(&brick_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&brick_texture.sampler)
                    }
                ]
            }
        );


        /* 
            Scene Kit Layout 
                [Lights, Camera]--Layout and Bind Group

         */

        /*
            Quick mock-up of a light until we need one. 
         */

        let dir = glam::Vec3::new(-0.5, -0.7, -1.0).normalize();
        let light_direction = [dir.x, dir.y, dir.z, 0.0]; 
        let light_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Light Buffer"),
                contents: bytemuck::cast_slice(&[light_direction]),
                usage: wgpu::BufferUsages::UNIFORM,
            }
        );


        let camera_buffer = &world.get_resource::<Camera>()
            .unwrap()
            .buffer;
        
        let scene_kit_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("Scene Kit Layout"),
                entries: &[
                    // Camera Entry 
                    wgpu::BindGroupLayoutEntry {
                        binding: 0, 
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer { 
                            ty: wgpu::BufferBindingType::Uniform, 
                            has_dynamic_offset: false, 
                            min_binding_size: None 
                        },
                        count: None 
                    },
                    // Light Entry 
                    wgpu::BindGroupLayoutEntry {
                        binding: 1, 
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer { 
                            ty: wgpu::BufferBindingType::Uniform, 
                            has_dynamic_offset: false, 
                            min_binding_size: None 
                        },
                        count: None 
                    }
                ]
            }
        );

        let scene_kit_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
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

                    }
                ]
            }
        );

        /*
            Render Pipeline definition 
         */

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SceneTree Pipeline Layout"),
            bind_group_layouts: &[&texture_layout, &scene_kit_layout],
            push_constant_ranges: &[]
        });


        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor{
            label: Some("Some Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::from(shader_source))
        });


        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SceneTree Pipeline"),
            layout : Some(&render_pipeline_layout), 
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[BrickVertex::desc(), BrickUniform::desc_instancing()],
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
                })]
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, 
                strip_index_format: None, 
                front_face: wgpu::FrontFace::Cw, 
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false, 
                conservative: false 
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
                alpha_to_coverage_enabled: false 
            },
            multiview: None, 
            cache: None             
        });



        world.insert_resource(Self {
            pipeline: render_pipeline,
            brick_vb: vb, 
            brick_ib: ib,
            brick_ibos: Vec::new(),
            scene_bg: scene_kit_group,
            texture_bg: texture_group,
            bricks: Vec::new()
        });
    }

    pub fn gen_bricks(mut st: ResMut<SceneTree>, state: Res<RenderState>, mut query: Query<BrickQueryReorder>) {
        let device = &state.device;

        let mut i = 0; 
        let mut brick_data: Vec<BrickUniform> = Vec::new();

        for mut brick in query.iter_mut() {
            brick_data.push(Brick::to_uniform(brick.brick, brick.position, brick.rotation, brick.size, brick.color));
            brick.buffer_index.0 = Some(i);
            i += 1;

        }

        let instance_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&brick_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            }
        );

        st.brick_ibos.push(instance_buffer);
        st.bricks = brick_data;
    }

    pub fn update_bricks(scene: Res<RenderState>, mut st: ResMut<SceneTree>, query: Query<BrickQuery, BrickFilterUpdate>) {
        #[allow(unused)]
        let device  = &scene.device;
        #[allow(unused)]
        let queue = &scene.queue;

        for brick in query.iter() {
            let index = brick.buffer_index.0.unwrap();

            if let Some(uniform) = st.bricks.get_mut(index as usize) {
                *uniform = Brick::to_uniform(brick.brick, brick.position, brick.rotation, brick.size, brick.color);
            }

            if let Some(buffer) = st.brick_ibos.first() {
                queue.write_buffer(buffer, 0, bytemuck::cast_slice(&st.bricks));
            }
        }
    }


    pub fn render(scene_tree: ResMut<SceneTree>, mut info: ResMut<RenderPassInfo>) {
        let pass = info 
            .pass
            .as_mut()
            .expect("SceneTree::render(), expected RenderPass");

        pass.set_pipeline(&scene_tree.pipeline);

        pass.set_bind_group(0, &scene_tree.texture_bg, &[]);
        pass.set_bind_group(1, &scene_tree.scene_bg, &[]);

        pass.set_vertex_buffer(0, scene_tree.brick_vb.slice(..));

        pass.set_index_buffer(scene_tree.brick_ib.slice(..), wgpu::IndexFormat::Uint16);


        // Remove in future, just having one instance buffer for now 
        assert!(scene_tree.brick_ibos.len() == 1);

        for buf in &scene_tree.brick_ibos {
            pass.set_vertex_buffer(1, buf.slice(..));
        }
        // Coupled with assert, this needs to be refactored once we "split up" scenes.
        pass.draw_indexed(0..36, 0, 0..scene_tree.bricks.len() as _); 


    }

}