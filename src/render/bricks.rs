/*
    Information about bricks relevant to the rendering engine. 
*/
use bytemuck::{Pod, Zeroable};
use crate::ecs::{
    bricks::Brick, 
    common::*,
};
use glam::Affine3A;

/*
    Implementing render-related stuff here. 
*/

impl Brick {
    pub fn to_uniform(
            _brick: &Brick,
            position: &Position,
            rotation: &Rotation,
            size: &Size, 
            color: &Color
        ) -> BrickUniform {
            let transform = Affine3A::from_scale_rotation_translation(
                size.0, 
                rotation.0, 
                position.0
            );

            let normals = transform
                .matrix3
                .inverse() 
                .transpose()
                .to_cols_array_2d();

            BrickUniform { 
                model: transform.to_cols_array_2d(), 
                normal: normals, 
                color: color.0, 
                size: size.0.to_array(), 
                stud_layout: 0x210000 // To-do, conversion func 
            }
        }
}


#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct BrickVertex {
    pub position: [f32; 3],
    pub normals: [f32; 3],
    pub tex_coords: [f32; 2],
    pub tex_scale: [u16; 2],
} 

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct BrickUniform {
    pub model: [[f32; 3]; 4], 
    pub normal: [[f32; 3]; 3],
    pub color: [u8; 4],
    pub size:  [f32; 3],
    pub stud_layout: u32,
}


impl BrickVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem::size_of;

        wgpu::VertexBufferLayout {
            array_stride: size_of::<BrickVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex, 
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0, 
                    shader_location: 0, 
                    format: wgpu::VertexFormat::Float32x3
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1, 
                    format: wgpu::VertexFormat::Float32x3,
                }, 
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 6]>() as wgpu::BufferAddress, 
                    shader_location: 2, 
                    format: wgpu::VertexFormat::Float32x2,
                }, 
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 3, 
                    format: wgpu::VertexFormat::Uint16x2,
                }
            ]
        }
    }
}


impl BrickUniform {
    pub fn desc_instancing() -> wgpu::VertexBufferLayout<'static> {
        use std::mem::size_of;

        wgpu::VertexBufferLayout {
            array_stride: size_of::<BrickUniform>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance, 
            attributes: &[
                // Affine transform 
                wgpu::VertexAttribute {
                    offset: 0, 
                    shader_location: 5, 
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 9]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x3,
                },
                //wgpu::VertexAttribute {
                //    offset: size_of::<[f32; 12]>() as wgpu::BufferAddress,
                 //   shader_location: 8,
                 //   format: wgpu::VertexFormat::Float32x4,
                //},
                // Normal
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 15]>() as wgpu::BufferAddress,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 18]>() as wgpu::BufferAddress,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Color 
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 21]>() as wgpu::BufferAddress,
                    shader_location: 12,
                    format: wgpu::VertexFormat::Unorm8x4,
                },
                // Size 
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 22]>() as wgpu::BufferAddress,
                    shader_location: 13,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Stud Layout 
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 25]>() as wgpu::BufferAddress,
                    shader_location: 14,
                    format: wgpu::VertexFormat::Uint32,
                },
            ]

        }
    }
}

pub const VERTICES: &[BrickVertex] = &[
    // Front face (Z+)
    BrickVertex { position: [-0.5,  0.5, 0.5], tex_coords: [0.0, 0.0], normals: [0.0, 0.0, 1.0], tex_scale: [0, 1]},
    BrickVertex { position: [ 0.5,  0.5, 0.5], tex_coords: [1.0, 0.0], normals: [0.0, 0.0, 1.0], tex_scale: [0, 1]},
    BrickVertex { position: [ 0.5, -0.5, 0.5], tex_coords: [1.0, 1.0], normals: [0.0, 0.0, 1.0], tex_scale: [0, 1]},
    BrickVertex { position: [-0.5, -0.5, 0.5], tex_coords: [0.0, 1.0], normals: [0.0, 0.0, 1.0], tex_scale: [0, 1]},


    // Right Face (X+)
    BrickVertex { position: [ 0.5,  0.5,  0.5], tex_coords: [0.0, 0.0], normals: [1.0, 0.0, 0.0], tex_scale: [2, 1]},
    BrickVertex { position: [ 0.5,  0.5, -0.5], tex_coords: [1.0, 0.0], normals: [1.0, 0.0, 0.0], tex_scale: [2, 1]},
    BrickVertex { position: [ 0.5, -0.5, -0.5], tex_coords: [1.0, 1.0], normals: [1.0, 0.0, 0.0], tex_scale: [2, 1]},
    BrickVertex { position: [ 0.5, -0.5,  0.5], tex_coords: [0.0, 1.0], normals: [1.0, 0.0, 0.0], tex_scale: [2, 1]},

    // Back Face (Z-)
    BrickVertex { position: [ 0.5,  0.5, -0.5], tex_coords: [0.0, 0.0], normals: [0.0, 0.0, -1.0], tex_scale: [0, 1]},
    BrickVertex { position: [-0.5,  0.5, -0.5], tex_coords: [1.0, 0.0], normals: [0.0, 0.0, -1.0], tex_scale: [0, 1]},
    BrickVertex { position: [-0.5, -0.5, -0.5], tex_coords: [1.0, 1.0], normals: [0.0, 0.0, -1.0], tex_scale: [0, 1]},
    BrickVertex { position: [ 0.5, -0.5, -0.5], tex_coords: [0.0, 1.0], normals: [0.0, 0.0, -1.0], tex_scale: [0, 1]},


    // Left Face (X-)
    BrickVertex { position: [ -0.5,  0.5, -0.5], tex_coords: [0.0, 0.0], normals: [-1.0, 0.0, 0.0], tex_scale: [2, 1]},
    BrickVertex { position: [ -0.5,  0.5,  0.5], tex_coords: [1.0, 0.0], normals: [-1.0, 0.0, 0.0], tex_scale: [2, 1]},
    BrickVertex { position: [ -0.5, -0.5,  0.5], tex_coords: [1.0, 1.0], normals: [-1.0, 0.0, 0.0], tex_scale: [2, 1]},
    BrickVertex { position: [ -0.5, -0.5, -0.5], tex_coords: [0.0, 1.0], normals: [-1.0, 0.0, 0.0], tex_scale: [2, 1]},

    // Top Face (Y+)
    BrickVertex { position: [  0.5,  0.5, -0.5], tex_coords: [0.0, 0.0], normals: [0.0, 1.0, 0.0], tex_scale: [2, 0]},
    BrickVertex { position: [  0.5,  0.5,  0.5], tex_coords: [1.0, 0.0], normals: [0.0, 1.0, 0.0], tex_scale: [2, 0]},
    BrickVertex { position: [ -0.5,  0.5,  0.5], tex_coords: [1.0, 1.0], normals: [0.0, 1.0, 0.0], tex_scale: [2, 0]},
    BrickVertex { position: [ -0.5,  0.5, -0.5], tex_coords: [0.0, 1.0], normals: [0.0, 1.0, 0.0], tex_scale: [2, 0]},

    // Bottom Face (Y-)
    BrickVertex { position: [  0.5, -0.5,  0.5], tex_coords: [0.0, 0.0], normals: [0.0, -1.0, 0.0], tex_scale: [2, 0]},
    BrickVertex { position: [  0.5, -0.5, -0.5], tex_coords: [1.0, 0.0], normals: [0.0, -1.0, 0.0], tex_scale: [2, 0]},
    BrickVertex { position: [  -0.5, -0.5, -0.5], tex_coords: [1.0, 1.0], normals: [0.0, -1.0, 0.0], tex_scale: [2, 0]},
    BrickVertex { position: [  -0.5, -0.5,  0.5], tex_coords: [0.0, 1.0], normals: [0.0, -1.0, 0.0], tex_scale: [2, 0]},
];

pub const INDICES: &[u16] = &[
    // Front face
    0, 1, 2, 0, 2, 3,
    // Back face
    4, 5, 6, 4, 6, 7,
    // Left face
    8, 9, 10, 8, 10, 11,
    // Right face
    12, 13, 14, 12, 14, 15,
    // Top face
    16, 17, 18, 16, 18, 19,
    // Bottom face
    20, 21, 22, 20, 22, 23,
];
