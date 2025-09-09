use crate::{
    common::types::HasPosition,
    ecs::{common::*, parts::*},
};
use bytemuck::{Pod, Zeroable};
use glam::{Affine3A, Mat4};

impl Part {
    pub fn to_instance(
        part: &Part,
        _studs: &StudInfo,
        position: &Position,
        rotation: &Rotation,
        size: &Size,
        color: &Color,
    ) -> PartInstance {
        let transform = Affine3A::from_scale_rotation_translation(size.0, rotation.0, position.0);

        let normals = transform.matrix3.inverse().transpose().to_cols_array_2d();

        let stud_layout: u32 = match part {
            Part::Brick => 0x001020,
            Part::Wedge => 0x000020,
            _ => 0,
        };

        PartInstance {
            model: transform.to_cols_array_2d(),
            normal: normals,
            color: color.0,
            size: size.0.to_array(),
            stud_layout: stud_layout,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct PartVertex {
    pub position: [f32; 3],
    pub normals: [f32; 3],
    pub tex_coords: [f32; 2],
    pub tex_scale: [u16; 2],
}

impl PartVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem::size_of;

        wgpu::VertexBufferLayout {
            array_stride: size_of::<PartVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Normals
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Tex Coords
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // Tex Scale
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Uint16x2,
                },
            ],
        }
    }
}

impl PartInstance {
    pub fn desc_instancing() -> wgpu::VertexBufferLayout<'static> {
        use std::mem::size_of;

        wgpu::VertexBufferLayout {
            array_stride: size_of::<PartInstance>() as wgpu::BufferAddress,
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
                // Normal matrix
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
            ],
        }
    }
}

impl HasPosition for PartVertex {
    fn get_position(&self) -> &[f32; 3] {
        &self.position
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct PartInstance {
    pub model: [[f32; 3]; 4],
    pub normal: [[f32; 3]; 3],
    pub color: [u8; 4],
    pub size: [f32; 3],
    pub stud_layout: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct PartUniform {
    pub model: [[f32; 4]; 4],
    pub normal: [[f32; 4]; 4],
    pub color: [f32; 4],
    pub size: [f32; 3],
    pub stud_layout: u32,
}

impl From<PartInstance> for PartUniform {
    fn from(value: PartInstance) -> Self {
        let model = Affine3A::from_cols_array_2d(&value.model);
        let model: Mat4 = model.into();
        let normal = model.inverse().transpose().to_cols_array_2d();

        let color = [
            value.color[0] as f32 / 255 as f32,
            value.color[1] as f32 / 255 as f32,
            value.color[2] as f32 / 255 as f32,
            value.color[3] as f32 / 255 as f32,
        ];

        Self {
            model: model.to_cols_array_2d(),
            normal: normal,
            color: color,
            size: value.size,
            stud_layout: value.stud_layout,
        }
    }
}
