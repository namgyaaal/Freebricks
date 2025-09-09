use std::ops::{Deref, DerefMut};

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

struct Plane {
    normal: Vec3,
    distance: f32,
}

impl Plane {
    const ZERO: Self = Self {
        normal: Vec3::ZERO,
        distance: 0.0,
    };

    fn new(p1: Vec3, norm: Vec3) -> Self {
        let normal = Vec3::normalize(norm);
        let distance = normal.dot(p1);

        Plane {
            normal: normal,
            distance: distance,
        }
    }

    fn get_signed_distance(&self, p: Vec3) -> f32 {
        Vec3::dot(self.normal, p) - self.distance
    }

    fn is_on_or_forward(&self, center: Vec3, extent: Vec3) -> bool {
        let r = extent.x * self.normal.x.abs()
            + extent.y * self.normal.y.abs()
            + extent.z * self.normal.z.abs();

        -r <= self.get_signed_distance(center)
    }
}

struct Frustum {
    top: Plane,
    bottom: Plane,

    left: Plane,
    right: Plane,

    near: Plane,
    far: Plane,
}

impl Frustum {
    const ZERO: Self = Self {
        top: Plane::ZERO,
        bottom: Plane::ZERO,
        left: Plane::ZERO,
        right: Plane::ZERO,
        near: Plane::ZERO,
        far: Plane::ZERO,
    };

    fn new(camera: &Camera, aspect: f32, fov_y: f32, z_near: f32, z_far: f32) -> Frustum {
        let half_v_size = z_far * f32::tan(fov_y * 0.5);
        let half_h_side = half_v_size * aspect;
        let front_mult_far = z_far * camera.front;

        Frustum {
            near: Plane::new(camera.position + z_near * camera.front, camera.front),
            far: Plane::new(camera.position + front_mult_far, -camera.front),
            right: Plane::new(
                camera.position,
                Vec3::cross(front_mult_far - camera.right * half_h_side, camera.up),
            ),
            left: Plane::new(
                camera.position,
                Vec3::cross(camera.up, front_mult_far + camera.right * half_h_side),
            ),
            top: Plane::new(
                camera.position,
                Vec3::cross(camera.right, front_mult_far - camera.up * half_h_side),
            ),
            bottom: Plane::new(
                camera.position,
                Vec3::cross(front_mult_far + camera.up * half_v_size, camera.right),
            ),
        }
    }
}

#[derive(Resource)]
pub struct Camera {
    pub buffer: wgpu::Buffer,
    pub position: Vec3,
    pub front: Vec3,
    pub up: Vec3,
    pub right: Vec3,

    frustum: Frustum,
    proj: Mat4,
    view: Mat4,
    /*
       We usually include camera bindgoup with a bindgroup layout that has lighting and other useful stuff.
       This is just the camera uniform, nothing more and nothing less.
       Useful for situatons like the debug rendering.
    */
    pub default_layout: wgpu::BindGroupLayout,
    pub default_group: wgpu::BindGroup,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    view_pos: [f32; 4],
}
impl Camera {
    pub fn init(mut commands: Commands, state: Res<RenderState>) {
        let device = &state.device;
        let config = &state.config;

        // Construct defaults
        let world_up = Vec3::new(0.0, 1.0, 0.0);

        let eye = Vec3::new(0.0, 10.0, -4.0);
        let target = Vec3::new(0.0, 3.0, 10.0);

        let front = (target - eye).normalize();
        let right = Vec3::cross(front, world_up).normalize();
        let up = Vec3::cross(right, front).normalize();

        let view = Mat4::look_at_rh(eye, eye + front, up);

        let aspect = config.width as f32 / config.height as f32;
        let proj = Mat4::perspective_rh(70.0_f32.to_radians(), aspect, 0.1, 800.0);
        let mat = OPENGL_TO_WGPU_MATRIX * proj * view;

        let uniform = CameraUniform {
            view_proj: (mat).to_cols_array_2d(),
            view_pos: [0.0, 5.0, 0.0, 0.0],
        };

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera Layout"),
            entries: &[
                // Camera Entry
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let camera_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Group"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let mut cam = Self {
            buffer: camera_buffer,
            position: eye,
            front: front,
            up: up,
            right: right,
            frustum: Frustum::ZERO,
            proj: proj,
            view: view,
            default_layout: camera_layout,
            default_group: camera_group,
        };
        cam.frustum = Frustum::new(&cam, aspect, 70.0_f32.to_radians(), 0.1, 800.0);

        commands.insert_resource(cam);
    }

    pub fn look_at(&mut self, eye: Vec3, target: Vec3) {
        let world_up = Vec3::new(0.0, 1.0, 0.0);

        let front = (target - eye).normalize();
        let right = Vec3::cross(front, world_up).normalize();
        let up = Vec3::cross(right, front).normalize();

        self.position = eye;
        self.view = Mat4::look_at_rh(eye, eye + front, up);
        self.front = front;
        self.right = right;
        self.up = up;
    }

    pub fn update(mut camera: ResMut<Camera>, state: Res<RenderState>, mut _counter: Local<f32>) {
        let camera = camera.deref_mut();
        let queue = &state.queue;

        // *counter += 0.01;

        //let x = counter.cos() * 20.0;
        //let z = counter.sin() * 20.0;

        //camera.look_at(camera.position, Vec3::new(x, 10.0, z));
        let aspect = state.config.width as f32 / state.config.height as f32;

        camera.frustum = Frustum::new(camera.deref(), aspect, 70.0_f32.to_radians(), 0.1, 800.0);

        camera.proj = Mat4::perspective_rh(70.0_f32.to_radians(), aspect, 0.1, 800.0);
        let mat = OPENGL_TO_WGPU_MATRIX * camera.proj * camera.view;

        let uniform = CameraUniform {
            view_proj: (mat).to_cols_array_2d(),
            view_pos: [camera.position.x, camera.position.y, camera.position.z, 0.0],
        };

        queue.write_buffer(&camera.buffer, 0, bytemuck::cast_slice(&[uniform]));
    }

    pub fn check_bounds(&self, center: Vec3, extent: Vec3) -> bool {
        self.frustum.bottom.is_on_or_forward(center, extent)
            && self.frustum.top.is_on_or_forward(center, extent)
            && self.frustum.near.is_on_or_forward(center, extent)
            && self.frustum.far.is_on_or_forward(center, extent)
            && self.frustum.left.is_on_or_forward(center, extent)
            && self.frustum.right.is_on_or_forward(center, extent)
    }
}
