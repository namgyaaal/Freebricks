use std::{collections::VecDeque, num::NonZero, sync::OnceLock};

use wgpu::{CommandEncoder, util::StagingBelt};

use crate::{
    ecs::parts::Part,
    render::{
        details::part_details::part_details_get,
        parts::{PartInstance, PartUniform},
    },
};

pub const MAX_INSTANCE_COUNT: usize = (u16::MAX / 4) as usize;
pub static UNIFORM_LAYOUT_SIZE: OnceLock<usize> = OnceLock::new();
pub static MAX_UNIFORM_COUNT: OnceLock<usize> = OnceLock::new();

pub struct ReadyBuffer {
    pub part_type: Part,
    pub len: usize,
    pub buffer: wgpu::Buffer,
    pub bind_and_offsets: Option<(wgpu::BindGroup, Vec<wgpu::DynamicOffset>)>,
}

pub struct PartQueue {
    unused_uniform_buffers: VecDeque<(wgpu::Buffer, wgpu::BindGroup)>,
    unused_instance_buffers: VecDeque<wgpu::Buffer>,

    uniform_belt: StagingBelt,
    instance_belt: StagingBelt,
    uniform_dirty: bool,
    instance_dirty: bool,

    ready_uniform_buffers: VecDeque<ReadyBuffer>,
    ready_instance_buffers: VecDeque<ReadyBuffer>,
}

impl PartQueue {
    const INSTANCE_SIZE: u64 = std::mem::size_of::<PartInstance>() as u64;
    const UNIFORM_SIZE: u64 = std::mem::size_of::<PartUniform>() as u64;

    pub fn new(device: &wgpu::Device) -> Self {
        // Uniform count and uniform layout size should be filled here.
        let uniform_count = *MAX_UNIFORM_COUNT.get_or_init(|| {
            let per_size = *UNIFORM_LAYOUT_SIZE.get_or_init(|| {
                let min_layout_size = device.limits().min_uniform_buffer_offset_alignment as usize;
                usize::max(min_layout_size, Self::UNIFORM_SIZE as usize)
            });
            let max_size = device.limits().max_uniform_buffer_binding_size as usize;
            max_size / per_size
        });

        let layout_size = *UNIFORM_LAYOUT_SIZE
            .get()
            .expect("Uniform layout should be filled in");

        let mut part_queue = PartQueue {
            unused_instance_buffers: VecDeque::new(),
            unused_uniform_buffers: VecDeque::new(),
            uniform_belt: StagingBelt::new((layout_size * uniform_count) as u64),
            instance_belt: StagingBelt::new(Self::INSTANCE_SIZE * MAX_INSTANCE_COUNT as u64),
            uniform_dirty: false,
            instance_dirty: false,
            ready_instance_buffers: VecDeque::new(),
            ready_uniform_buffers: VecDeque::new(),
        };

        part_queue
            .unused_instance_buffers
            .push_back(part_queue.new_instance_buffer(device));

        // Allocate a handful of uniform buffers
        for _ in 0..5 {
            let buffer_and_bind_group = part_queue.new_uniform_buffer(device);

            part_queue
                .unused_uniform_buffers
                .push_back(buffer_and_bind_group);
        }
        part_queue
    }

    fn new_uniform_buffer(&self, device: &wgpu::Device) -> (wgpu::Buffer, wgpu::BindGroup) {
        let uniform_count = *MAX_UNIFORM_COUNT
            .get()
            .expect("Uniform count should be filled");

        let layout_size = *UNIFORM_LAYOUT_SIZE
            .get()
            .expect("Uniform layout should be filled in");

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Part Uniform Buffer"),
            size: (uniform_count * layout_size) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Part Uniform Bind Group"),
            layout: &part_details_get().part_uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new(Self::UNIFORM_SIZE as u64), // Give it entire layout for now
                }),
            }],
        });
        (buffer, bind_group)
    }

    fn new_instance_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        let size = Self::INSTANCE_SIZE * MAX_INSTANCE_COUNT as u64;

        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Part Instance Buffer"),
            size: size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn get_uniform_buffer(&mut self, device: &wgpu::Device) -> (wgpu::Buffer, wgpu::BindGroup) {
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

    pub fn map_slice(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut CommandEncoder,
        part_type: Part,
        uniforms: &[PartInstance],
    ) {
        let max_uniform = *MAX_UNIFORM_COUNT
            .get()
            .expect("Uniform count should be filled");
        if uniforms.len() <= max_uniform * 4 {
            // Uniform

            // Uniform Buffer Objects work a bit differently than Instance Buffer Objects, they require a padding of 256 bytes,
            //  so we can't naively just copy them here. we will have to get a view then iterate.
            let layout_size = *UNIFORM_LAYOUT_SIZE
                .get()
                .expect("Uniform layout should be filled in");

            for chunk in uniforms.rchunks(max_uniform) {
                let (buffer, bind_group) = self.get_uniform_buffer(device);
                let view_size = NonZero::new((layout_size * chunk.len()) as u64)
                    .expect("View size didn't construct");
                let mut view = self
                    .uniform_belt
                    .write_buffer(encoder, &buffer, 0, view_size, device);

                // re-alignment to uniform buffer alignment size
                for i in 0..chunk.len() {
                    let uniform = PartUniform::from(chunk[i]);
                    view[i * layout_size..i * layout_size + Self::UNIFORM_SIZE as usize]
                        .copy_from_slice(&bytemuck::cast_slice(&[uniform]));
                }

                let offsets: Vec<wgpu::DynamicOffset> = (0..chunk.len())
                    .map(|i| (i * layout_size) as wgpu::DynamicOffset)
                    .collect();

                self.ready_uniform_buffers.push_back(ReadyBuffer {
                    part_type: part_type,
                    len: chunk.len(),
                    buffer: buffer,
                    bind_and_offsets: Some((bind_group, offsets)),
                });
            }

            self.uniform_dirty = true;
        } else {
            // Instancing
            for chunk in uniforms.rchunks(MAX_INSTANCE_COUNT) {
                let buffer = self.get_instance_buffer(device);

                let max_len: u64 = Self::INSTANCE_SIZE * usize::max(chunk.len(), 1024) as u64;
                let view_size = NonZero::new(max_len).expect("Wut");
                let mut view = self
                    .instance_belt
                    .write_buffer(encoder, &buffer, 0, view_size, device);

                let end = chunk.len() * Self::INSTANCE_SIZE as usize;
                view[0..end].copy_from_slice(bytemuck::cast_slice(chunk));
                self.ready_instance_buffers.push_back(ReadyBuffer {
                    part_type: part_type,
                    len: chunk.len(),
                    buffer: buffer,
                    bind_and_offsets: None,
                });
            }
            self.instance_dirty = true;
        }
    }

    pub fn submit(&mut self) {
        if self.uniform_dirty {
            self.uniform_belt.finish();
        }
        if self.instance_dirty {
            self.instance_belt.finish();
        }
    }
    pub fn recall(&mut self) {
        if self.uniform_dirty {
            self.uniform_belt.recall();
            self.uniform_dirty = false;
        }
        if self.instance_dirty {
            self.instance_belt.recall();
            self.instance_dirty = false;
        }
    }

    pub fn drain_instances<F>(&mut self, mut callback: F)
    where
        F: FnMut(&ReadyBuffer),
    {
        while let Some(ready_buffer) = self.ready_instance_buffers.pop_front() {
            callback(&ready_buffer);

            match ready_buffer {
                ReadyBuffer { buffer, .. } => {
                    self.unused_instance_buffers.push_back(buffer);
                }
            }
        }
    }

    pub fn drain_uniforms<F>(&mut self, mut callback: F)
    where
        F: FnMut(&ReadyBuffer),
    {
        while let Some(ready_buffer) = self.ready_uniform_buffers.pop_front() {
            callback(&ready_buffer);
            match ready_buffer {
                ReadyBuffer {
                    buffer,
                    bind_and_offsets,
                    ..
                } => {
                    let (bind_group, _) =
                        bind_and_offsets.expect("Bind Groups should accompany uniform buffer");

                    self.unused_uniform_buffers.push_back((buffer, bind_group));
                }
            }
        }
    }
}
