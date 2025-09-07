/*
    Information about bricks relevant to the rendering engine.
*/
use crate::{
    common::types::HasPosition,
    ecs::{common::*, parts::*},
};
use bevy_platform::collections::HashMap;
use bytemuck::{Pod, Zeroable};
use glam::{Affine3A, Vec3};
use std::{
    f32::{self, consts::PI},
    sync::{LazyLock, OnceLock},
};
use wgpu::util::DeviceExt;

static PART_BUFFERS: OnceLock<HashMap<Part, (wgpu::Buffer, wgpu::Buffer)>> = OnceLock::new();
pub fn part_buffer_init(device: &wgpu::Device) {
    let mut map: HashMap<Part, (wgpu::Buffer, wgpu::Buffer)> = HashMap::new();
    // Brick
    let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Brick Vertex Buffer"),
        contents: bytemuck::cast_slice(BRICK_VERTICES),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Brick Index Buffer"),
        contents: bytemuck::cast_slice(BRICK_INDICES),
        usage: wgpu::BufferUsages::INDEX,
    });
    map.insert(Part::Brick, (vb, ib));

    // Wedge
    let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Brick Vertex Buffer"),
        contents: bytemuck::cast_slice(WEDGE_VERTICES),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Brick Index Buffer"),
        contents: bytemuck::cast_slice(WEDGE_INDICES),
        usage: wgpu::BufferUsages::INDEX,
    });
    map.insert(Part::Wedge, (vb, ib));

    // Ball
    let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Brick Vertex Buffer"),
        contents: bytemuck::cast_slice(&*BALL_VERTICES),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Brick Index Buffer"),
        contents: bytemuck::cast_slice(&*BALL_INDICES),
        usage: wgpu::BufferUsages::INDEX,
    });
    map.insert(Part::Wedge, (vb, ib));

    PART_BUFFERS
        .set(map)
        .expect("Can't call part_buffer_init() twice");
}

pub fn part_buffer_fetch(part_type: Part) -> (&'static wgpu::Buffer, &'static wgpu::Buffer) {
    let map = PART_BUFFERS
        .get()
        .expect("Called part_buffer_fetch() before part_buffer_init()");

    let pair = map.get(&part_type).expect("Part not included");

    (&pair.0, &pair.1)
}

impl Part {
    pub fn to_uniform(
        part: &Part,
        studs: &StudInfo,
        position: &Position,
        rotation: &Rotation,
        size: &Size,
        color: &Color,
    ) -> PartUniform {
        let transform = Affine3A::from_scale_rotation_translation(size.0, rotation.0, position.0);

        let normals = transform.matrix3.inverse().transpose().to_cols_array_2d();

        let stud_layout: u32 = match part {
            Part::Brick => 0x001020,
            Part::Wedge => 0x000020,
            _ => 0,
        };

        PartUniform {
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

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct PartUniform {
    pub model: [[f32; 3]; 4],
    pub normal: [[f32; 3]; 3],
    pub color: [u8; 4],
    pub size: [f32; 3],
    pub stud_layout: u32,
}

impl HasPosition for PartVertex {
    fn get_position(&self) -> &[f32; 3] {
        &self.position
    }
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

impl PartUniform {
    pub fn desc_instancing() -> wgpu::VertexBufferLayout<'static> {
        use std::mem::size_of;

        wgpu::VertexBufferLayout {
            array_stride: size_of::<PartUniform>() as wgpu::BufferAddress,
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

pub const BRICK_VERTICES: &[PartVertex] = &[
    // Front face (Z+)
    PartVertex {
        position: [-0.5, 0.5, 0.5],
        tex_coords: [0.0, 0.0],
        normals: [0.0, 0.0, 1.0],
        tex_scale: [0, 1],
    },
    PartVertex {
        position: [0.5, 0.5, 0.5],
        tex_coords: [1.0, 0.0],
        normals: [0.0, 0.0, 1.0],
        tex_scale: [0, 1],
    },
    PartVertex {
        position: [0.5, -0.5, 0.5],
        tex_coords: [1.0, 1.0],
        normals: [0.0, 0.0, 1.0],
        tex_scale: [0, 1],
    },
    PartVertex {
        position: [-0.5, -0.5, 0.5],
        tex_coords: [0.0, 1.0],
        normals: [0.0, 0.0, 1.0],
        tex_scale: [0, 1],
    },
    // Bottom Face (Y-)
    PartVertex {
        position: [0.5, -0.5, 0.5],
        tex_coords: [0.0, 0.0],
        normals: [0.0, -1.0, 0.0],
        tex_scale: [2, 0],
    },
    PartVertex {
        position: [0.5, -0.5, -0.5],
        tex_coords: [1.0, 0.0],
        normals: [0.0, -1.0, 0.0],
        tex_scale: [2, 0],
    },
    PartVertex {
        position: [-0.5, -0.5, -0.5],
        tex_coords: [1.0, 1.0],
        normals: [0.0, -1.0, 0.0],
        tex_scale: [2, 0],
    },
    PartVertex {
        position: [-0.5, -0.5, 0.5],
        tex_coords: [0.0, 1.0],
        normals: [0.0, -1.0, 0.0],
        tex_scale: [2, 0],
    },
    // Back Face (Z-)
    PartVertex {
        position: [0.5, 0.5, -0.5],
        tex_coords: [0.0, 0.0],
        normals: [0.0, 0.0, -1.0],
        tex_scale: [0, 1],
    },
    PartVertex {
        position: [-0.5, 0.5, -0.5],
        tex_coords: [1.0, 0.0],
        normals: [0.0, 0.0, -1.0],
        tex_scale: [0, 1],
    },
    PartVertex {
        position: [-0.5, -0.5, -0.5],
        tex_coords: [1.0, 1.0],
        normals: [0.0, 0.0, -1.0],
        tex_scale: [0, 1],
    },
    PartVertex {
        position: [0.5, -0.5, -0.5],
        tex_coords: [0.0, 1.0],
        normals: [0.0, 0.0, -1.0],
        tex_scale: [0, 1],
    },
    // Top Face (Y+)
    PartVertex {
        position: [0.5, 0.5, -0.5],
        tex_coords: [0.0, 0.0],
        normals: [0.0, 1.0, 0.0],
        tex_scale: [2, 0],
    },
    PartVertex {
        position: [0.5, 0.5, 0.5],
        tex_coords: [1.0, 0.0],
        normals: [0.0, 1.0, 0.0],
        tex_scale: [2, 0],
    },
    PartVertex {
        position: [-0.5, 0.5, 0.5],
        tex_coords: [1.0, 1.0],
        normals: [0.0, 1.0, 0.0],
        tex_scale: [2, 0],
    },
    PartVertex {
        position: [-0.5, 0.5, -0.5],
        tex_coords: [0.0, 1.0],
        normals: [0.0, 1.0, 0.0],
        tex_scale: [2, 0],
    },
    // Right Face (X+)
    PartVertex {
        position: [0.5, 0.5, 0.5],
        tex_coords: [0.0, 0.0],
        normals: [1.0, 0.0, 0.0],
        tex_scale: [2, 1],
    },
    PartVertex {
        position: [0.5, 0.5, -0.5],
        tex_coords: [1.0, 0.0],
        normals: [1.0, 0.0, 0.0],
        tex_scale: [2, 1],
    },
    PartVertex {
        position: [0.5, -0.5, -0.5],
        tex_coords: [1.0, 1.0],
        normals: [1.0, 0.0, 0.0],
        tex_scale: [2, 1],
    },
    PartVertex {
        position: [0.5, -0.5, 0.5],
        tex_coords: [0.0, 1.0],
        normals: [1.0, 0.0, 0.0],
        tex_scale: [2, 1],
    },
    // Left Face (X-)
    PartVertex {
        position: [-0.5, 0.5, -0.5],
        tex_coords: [0.0, 0.0],
        normals: [-1.0, 0.0, 0.0],
        tex_scale: [2, 1],
    },
    PartVertex {
        position: [-0.5, 0.5, 0.5],
        tex_coords: [1.0, 0.0],
        normals: [-1.0, 0.0, 0.0],
        tex_scale: [2, 1],
    },
    PartVertex {
        position: [-0.5, -0.5, 0.5],
        tex_coords: [1.0, 1.0],
        normals: [-1.0, 0.0, 0.0],
        tex_scale: [2, 1],
    },
    PartVertex {
        position: [-0.5, -0.5, -0.5],
        tex_coords: [0.0, 1.0],
        normals: [-1.0, 0.0, 0.0],
        tex_scale: [2, 1],
    },
];

pub const BRICK_INDICES: &[u16] = &[
    0, 1, 2, 0, 2, 3, // Front
    4, 5, 6, 4, 6, 7, // Bottom
    8, 9, 10, 8, 10, 11, // Back
    12, 13, 14, 12, 14, 15, // Top
    16, 17, 18, 16, 18, 19, // Right
    20, 21, 22, 20, 22, 23, // Left
];

pub const WEDGE_VERTICES: &[PartVertex] = &[
    // Front face (Z+)
    PartVertex {
        position: [-0.5, 0.5, 0.5],
        tex_coords: [0.0, 0.0],
        normals: [0.0, 0.0, 1.0],
        tex_scale: [0, 1],
    },
    PartVertex {
        position: [0.5, 0.5, 0.5],
        tex_coords: [1.0, 0.0],
        normals: [0.0, 0.0, 1.0],
        tex_scale: [0, 1],
    },
    PartVertex {
        position: [0.5, -0.5, 0.5],
        tex_coords: [1.0, 1.0],
        normals: [0.0, 0.0, 1.0],
        tex_scale: [0, 1],
    },
    PartVertex {
        position: [-0.5, -0.5, 0.5],
        tex_coords: [0.0, 1.0],
        normals: [0.0, 0.0, 1.0],
        tex_scale: [0, 1],
    },
    // Bottom Face (Y-)
    PartVertex {
        position: [0.5, -0.5, 0.5],
        tex_coords: [0.0, 0.0],
        normals: [0.0, -1.0, 0.0],
        tex_scale: [2, 0],
    },
    PartVertex {
        position: [0.5, -0.5, -0.5],
        tex_coords: [1.0, 0.0],
        normals: [0.0, -1.0, 0.0],
        tex_scale: [2, 0],
    },
    PartVertex {
        position: [-0.5, -0.5, -0.5],
        tex_coords: [1.0, 1.0],
        normals: [0.0, -1.0, 0.0],
        tex_scale: [2, 0],
    },
    PartVertex {
        position: [-0.5, -0.5, 0.5],
        tex_coords: [0.0, 1.0],
        normals: [0.0, -1.0, 0.0],
        tex_scale: [2, 0],
    },
    // Wedge (Z-)
    // tex_scale is redundant since no studs (TODO: something with decals?)
    PartVertex {
        position: [0.5, 0.5, 0.5],
        tex_coords: [0.0, 0.0],
        normals: [0.0, 0.707, -0.707],
        tex_scale: [0, 1],
    },
    PartVertex {
        position: [-0.5, 0.5, 0.5],
        tex_coords: [1.0, 0.0],
        normals: [0.0, 0.707, -0.707],
        tex_scale: [0, 1],
    },
    PartVertex {
        position: [-0.5, -0.5, -0.5],
        tex_coords: [1.0, 1.0],
        normals: [0.0, 0.707, -0.707],
        tex_scale: [0, 1],
    },
    PartVertex {
        position: [0.5, -0.5, -0.5],
        tex_coords: [0.0, 1.0],
        normals: [0.0, 0.707, -0.707],
        tex_scale: [0, 1],
    },
    // Right Face (X+)
    PartVertex {
        position: [0.5, 0.5, 0.5],
        tex_coords: [0.0, 0.0],
        normals: [1.0, 0.0, 0.0],
        tex_scale: [2, 1],
    },
    PartVertex {
        position: [0.5, -0.5, -0.5],
        tex_coords: [1.0, 1.0],
        normals: [1.0, 0.0, 0.0],
        tex_scale: [2, 1],
    },
    PartVertex {
        position: [0.5, -0.5, 0.5],
        tex_coords: [0.0, 1.0],
        normals: [1.0, 0.0, 0.0],
        tex_scale: [2, 1],
    },
    // Left Face (X-)
    PartVertex {
        position: [-0.5, 0.5, 0.5],
        tex_coords: [1.0, 0.0],
        normals: [-1.0, 0.0, 0.0],
        tex_scale: [2, 1],
    },
    PartVertex {
        position: [-0.5, -0.5, 0.5],
        tex_coords: [1.0, 1.0],
        normals: [-1.0, 0.0, 0.0],
        tex_scale: [2, 1],
    },
    PartVertex {
        position: [-0.5, -0.5, -0.5],
        tex_coords: [0.0, 1.0],
        normals: [-1.0, 0.0, 0.0],
        tex_scale: [2, 1],
    },
];

pub const WEDGE_INDICES: &[u16] = &[
    0, 1, 2, 0, 2, 3, // Front
    4, 5, 6, 4, 6, 7, // Bottom
    8, 9, 10, 8, 10, 11, // Wedge
    12, 13, 14, // Right
    15, 16, 17, // Left
];

pub static BALL_VERTICES: LazyLock<Vec<PartVertex>> = LazyLock::new(|| {
    // https://www.songho.ca/opengl/gl_sphere.html
    let sectors = 36; // Longitudes 
    let stacks = 18; // Latitudes 

    let sector_step = 2.0 * PI / sectors as f32;
    let stack_step = PI / stacks as f32;

    let mut vertices: Vec<PartVertex> = Vec::new();

    for i in 0..=stacks {
        let stack_angle = (PI / 2.0) - (i as f32 * stack_step); // [pi/2 .. -pi/2]
        let xy = stack_angle.cos();
        let z = stack_angle.sin();

        // add (sector_count + 1) vertices per stack
        // first and last vertices have same position and normals but different texcoords
        for j in 0..=sectors {
            let sector_angle = j as f32 * sector_step; // [0 .. 2pi]
            let x = xy * sector_angle.cos();
            let y = xy * sector_angle.sin();

            // Scale by 0.5 for positions
            // Not sure texcoords are what we want, just do what he uses for now.
            let vertex = PartVertex {
                position: [x / 2.0, y / 2.0, z / 2.0],
                normals: [x, y, z],
                tex_coords: [0.0, 0.0],
                tex_scale: [0, 0],
            };
            vertices.push(vertex);
        }
    }

    vertices
});

pub static BALL_INDICES: LazyLock<Vec<u16>> = LazyLock::new(|| {
    // https://www.songho.ca/opengl/gl_sphere.html
    let sectors = 36; // Longitudes 
    let stacks = 18; // Latitudes 

    let mut indices: Vec<u16> = Vec::new();

    for i in 0..stacks {
        let mut k1 = i * (sectors + 1); // Beginning of current stack
        let mut k2 = k1 + sectors + 1; // Beginning of next stack

        for _j in 0..sectors {
            if i != 0 {
                indices.extend([k1, k1 + 1, k2].iter());
            }

            if i != (stacks - 1) {
                indices.extend([k1 + 1, k2 + 1, k2].iter());
            }

            k1 += 1;
            k2 += 1;
        }
    }
    indices
});
