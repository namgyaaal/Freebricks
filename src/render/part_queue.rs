use std::{
    collections::VecDeque,
    num::NonZero,
    sync::{LazyLock, OnceLock},
};

use wgpu::{CommandEncoder, util::StagingBelt};

use crate::{ecs::parts::Part, render::parts::PartUniform};

const MAX_INSTANCE_COUNT: usize = u16::MAX as usize / 4;
const MAX_CHUNK_SIZE: usize = MAX_INSTANCE_COUNT / 2;

static MAX_UNIFORM_COUNT: OnceLock<usize> = OnceLock::new();

struct PartQueue {
    unused_uniform_buffers: VecDeque<wgpu::Buffer>,
    unused_instance_buffers: VecDeque<wgpu::Buffer>,

    uniform_belt: StagingBelt,
    instance_belt: StagingBelt,
    uniform_dirty: bool,
    instancing_dirty: bool,

    pub ready_uniform_buffers: VecDeque<(Part, usize, wgpu::Buffer)>,
    pub ready_instance_buffers: VecDeque<(Part, usize, wgpu::Buffer)>,
}

impl PartQueue {
    pub fn new(device: &wgpu::Device) -> Self {
        let size = (std::mem::size_of::<PartUniform>() * MAX_INSTANCE_COUNT) as u64;
        let uniform_count = *MAX_UNIFORM_COUNT.get_or_init(|| {
            let max_size = device.limits().max_uniform_buffer_binding_size as usize;
            let u_size = std::mem::size_of::<PartUniform>();

            u_size / max_size
        });

        let mut part_queue = PartQueue {
            unused_instance_buffers: VecDeque::new(),
            unused_uniform_buffers: VecDeque::new(),
            uniform_belt: StagingBelt::new(size * uniform_count as u64),
            instance_belt: StagingBelt::new(size * MAX_CHUNK_SIZE as u64),
            uniform_dirty: false,
            instancing_dirty: false,
            ready_instance_buffers: VecDeque::new(),
            ready_uniform_buffers: VecDeque::new(),
        };

        part_queue
            .unused_instance_buffers
            .push_back(part_queue.new_instance_buffer(device));

        // Get a handful of uniform buffers
        for _ in 0..5 {
            part_queue
                .unused_uniform_buffers
                .push_back(part_queue.new_uniform_buffer(device));
        }
        part_queue
    }

    fn new_uniform_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        let size = *MAX_UNIFORM_COUNT
            .get()
            .expect("Uniform count should be filled");
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Part Uniform Buffer"),
            size: (size * std::mem::size_of::<PartUniform>()) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn new_instance_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        let size = (std::mem::size_of::<PartUniform>() * MAX_INSTANCE_COUNT) as u64;

        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Part Instance Buffer"),
            size: size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn get_uniform_buffer(&mut self, device: &wgpu::Device) -> wgpu::Buffer {
        if let Some(buffer) = self.unused_uniform_buffers.pop_front() {
            buffer
        } else {
            self.new_uniform_buffer(device)
        }
    }

    fn get_instance_buffer(&mut self, device: &wgpu::Device) -> wgpu::Buffer {
        if let Some(buffer) = self.unused_instance_buffers.pop_front() {
            buffer
        } else {
            self.new_instance_buffer(device)
        }
    }

    fn map_slice(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut CommandEncoder,
        part_type: Part,
        uniforms: &[PartUniform],
    ) {
        let struct_size = std::mem::size_of::<PartUniform>();
        let max_uniform = *MAX_UNIFORM_COUNT
            .get()
            .expect("Uniform count should be filled");

        if uniforms.len() <= max_uniform * 4 {
            // Uniform
            for chunk in uniforms.rchunks(max_uniform) {
                let buffer = self.get_uniform_buffer(device);
                let view_size = NonZero::new((chunk.len() * struct_size) as u64).expect("Wut");
                let mut view = self
                    .uniform_belt
                    .write_buffer(encoder, &buffer, 0, view_size, device);

                let end = chunk.len() * struct_size;
                view[0..end].copy_from_slice(bytemuck::cast_slice(chunk));
                self.ready_uniform_buffers
                    .push_back((part_type, chunk.len(), buffer));
            }
            self.uniform_dirty = true;
        } else {
            // Instancing
            for chunk in uniforms.rchunks(MAX_INSTANCE_COUNT) {
                let buffer = self.get_instance_buffer(device);
                let view_size = NonZero::new((chunk.len() * struct_size) as u64).expect("Wut");
                let mut view = self
                    .uniform_belt
                    .write_buffer(encoder, &buffer, 0, view_size, device);

                let end = chunk.len() * struct_size;
                view[0..end].copy_from_slice(bytemuck::cast_slice(chunk));
                self.ready_instance_buffers
                    .push_back((part_type, chunk.len(), buffer));
            }
            self.instancing_dirty = true;
        }
    }

    fn submit(&mut self) {
        if self.uniform_dirty {
            self.uniform_belt.finish();
        }
        if self.instancing_dirty {
            self.instance_belt.finish();
        }
    }
}
