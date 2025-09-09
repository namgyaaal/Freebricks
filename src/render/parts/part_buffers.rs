use std::{
    f32::consts::PI,
    sync::{LazyLock, OnceLock},
};

use bevy_platform::collections::HashMap;
use wgpu::util::DeviceExt;

use crate::ecs::parts::Part;
use crate::render::parts::part_formats::PartVertex;

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
    map.insert(Part::Ball, (vb, ib));

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

pub fn part_index_count(part_type: Part) -> usize {
    match part_type {
        Part::Brick => 36,
        Part::Wedge => 24,
        Part::Ball => BALL_INDICES.len(),
        _ => panic!("part_index_count() not implemented"),
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
