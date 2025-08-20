use bevy_ecs::prelude::*;
use glam::{Mat4, Vec3, Vec4};
use wgpu::util::DeviceExt;

use crate::render::render_state::RenderState;

use bytemuck::{Pod, Zeroable};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: Mat4 = Mat4::from_cols(
    Vec4::new(1.0, 0.0, 0.0, 0.0),
    Vec4::new(0.0, 1.0, 0.0, 0.0),
    Vec4::new(0.0, 0.0, 0.5, 0.0),
    Vec4::new(0.0, 0.0, 0.5, 1.0),
);


#[derive(Resource)]
pub struct Camera {
    pub buffer: wgpu::Buffer,
    pub proj: Mat4, 
    pub view: Mat4,
    /*
        We usually include camera bindgoup with a bindgroup layout that has lighting and other useful stuff. 
        This is just the camera uniform, nothing more and nothing less. 
        Useful for situatons like the debug rendering.
     */
    pub default_layout: wgpu::BindGroupLayout,
    pub default_group: wgpu::BindGroup
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    view_pos: [f32; 4]
}

impl Camera {

    pub fn init(world: &mut World) {
        let render_state = world.get_resource::<RenderState>()
            .unwrap();

        let device = &render_state.device;
        let config = &render_state.config;

        let view = Mat4::look_at_rh(
            Vec3::new(30.0, 30.0, 30.0),
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );

        let proj = Mat4::perspective_rh(
            45.0, 
            config.width as f32 / config.height as f32, 
            0.1, 
            800.0
        );
        let mat = proj * view;

        let uniform = CameraUniform {
            view_proj: (mat).to_cols_array_2d(), 
            view_pos: [30.0, 30.0, 30.0, 0.0],
        };

        let camera_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
            }
        );   

 
        let camera_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Layout"),
                entries: &[
                    // Camera Entry 
                    wgpu::BindGroupLayoutEntry {
                        binding: 0, 
                        visibility: wgpu::ShaderStages::VERTEX,
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

        let camera_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some("Camera Group"),
                layout: &camera_layout, 
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: camera_buffer.as_entire_binding(), 
                    }
                ]
            }
        );



        world.insert_resource(Self {
            buffer: camera_buffer,
            proj: proj, 
            view: view,
            default_layout: camera_layout,
            default_group: camera_group
        });
    }
}