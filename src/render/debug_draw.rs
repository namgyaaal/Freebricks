use bevy_ecs::{prelude::*};
use rapier3d::prelude::*;
use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use std::{borrow::Cow, fmt::Debug, mem::size_of};

use crate::{common::{asset_cache::{AssetCache}}, render::{camera::Camera, render_state::{RenderPassInfo, RenderState}, texture::Texture}};


#[derive(Resource)]
pub struct DebugDraw {
    pipeline: wgpu::RenderPipeline,
    buffer: wgpu::Buffer, 
    lines: Vec<Vec3>,
}

impl DebugDraw {
    pub fn init(world: &mut World) {
        let asset_cache = world.get_resource::<AssetCache>()
            .unwrap();

        let shader_source = asset_cache
            .get_shader("shaders/debug_draw.wgsl")
            .expect("Couldn't load debug shader");


     

        let render_state = world.get_resource::<RenderState>()
            .expect("DebugDraw::init(), expected Render State");

        let camera = world.get_resource::<Camera>() 
            .expect("DebugDraw::init(), expected Camera");

        let device = &render_state.device;
        let config = &render_state.config;

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Debug Line Data"),
            size: size_of::<DebugVertex>() as u64 * (1024 * 1024),  // Should be good enough. 
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false 
        });



        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Debug Pipeline Layout"),
            bind_group_layouts: &[&camera.default_layout],
            push_constant_ranges: &[]
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Debug Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::from(shader_source)).into()
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Debug Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState { 
                module: &shader, 
                entry_point: Some("vs_main"),
                buffers: &[DebugVertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default()
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
                topology: wgpu::PrimitiveTopology::LineList, 
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
                depth_compare: wgpu::CompareFunction::Less, // 1.
                stencil: wgpu::StencilState::default(), // 2.
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


        world.insert_resource(DebugDraw {
            pipeline: pipeline, 
            buffer: buffer,
            lines: Vec::new()
        });
    }

    pub fn render(
        mut debug_draw: ResMut<DebugDraw>, 
        mut info: ResMut<RenderPassInfo>, 
        state: Res<RenderState>,
        camera: Res<Camera>
    ) {    
        let queue = &state.queue;
        let pass = info.pass
            .as_mut()
            .expect("DebugDraw::render(),  Render pass expected");
        if debug_draw.lines.len() == 0 {
            return;
        }

        let vertex_data: Vec<DebugVertex> = debug_draw
            .lines 
            .iter()
            .map(|v| DebugVertex {position: v.to_array()})
            .collect();

        queue.write_buffer(&debug_draw.buffer, 0, bytemuck::cast_slice(&vertex_data));
        pass.set_pipeline(&debug_draw.pipeline);
        pass.set_bind_group(0, &camera.default_group, &[]);

        let vertex_size = std::mem::size_of::<DebugVertex>() as u64;
        let buf_size = debug_draw.lines.len() as u64 * vertex_size;
        pass.set_vertex_buffer(0, debug_draw.buffer.slice(0..buf_size));
        pass.draw(0..debug_draw.lines.len() as u32, 0..1);



       debug_draw.lines.clear();
    }
}

impl DebugRenderBackend for DebugDraw {
    fn draw_line(
        &mut self, 
        _object: DebugRenderObject, 
        a: Point<f32>,
        b: Point<f32>,
        _color: DebugColor) 
    {
        self.lines.push(Vec3::new(a.x, a.y, a.z));
        self.lines.push(Vec3::new(b.x, b.y, b.z));
    }
    
}



#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct DebugVertex {
    pub position: [f32; 3],
}

impl DebugVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<DebugVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex, 
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0, 
                    shader_location: 0, 
                    format: wgpu::VertexFormat::Float32x3
                }
            ]
        }
    }
}