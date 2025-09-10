use std::{collections::VecDeque, marker::PhantomData, num::NonZero};

use wgpu::{CommandEncoder, util::StagingBelt};

use crate::{
    ecs::parts::Part,
    render::parts::{
        part_details::*,
        part_formats::{PartInstance, PartUniform},
    },
};
use bytemuck::{Pod, Zeroable};

pub type DefaultPartQueue = PartQueue<(Part, bool), PartInstance, PartUniform>;

const MAX_INSTANCE_COUNT: usize = (u16::MAX / 4) as usize;

/// Ready Buffer is exposed to whatever rendering process uses the PartQueue to provide extra
///     information to aid in the rendering process.
///
/// Because all buffers used by the PartQueue are constant in size and agnostic to vertex or index data
///     (since they are only used as instance buffers or uniform buffer objects), information such as
///     length or any generic data is useful to be exposed such that aforementioned issues can be handled
///     properly.
pub struct ReadyBuffer<T: Copy> {
    /// Key used to identify a ReadyBuffer (e.g., transparency and part type).
    pub key: T,
    /// How many per-object instances are stored
    pub len: usize,
    pub buffer: wgpu::Buffer,
    /// Used for when rendering as a uniform instead of instancing.
    ///
    /// Contains a bindgroup and list of offsets. Render by iterating through list and
    ///     continually shifting dynamic offset.
    pub bind_and_offsets: Option<(wgpu::BindGroup, Vec<wgpu::DynamicOffset>)>,
}

/// PartQueue solves there being a lot of objects with variations in vertex/index data
///     but not position by internally handling staging belts that are shared by
///     all possible variations.
///
/// It also handles whether or not they use instance buffers or with uniform buffers and breaking them into multiple
///     buffers if they are large.
///
/// For instance, with Parts we have bricks, wedges and balls. They all share the same underlying per-object data
///     (PartUniform, PartInstance). PartQueue can handle mapping their data to the appropiate.
///
/// Generic over
///     (1) what identifies the buffers when exposed to whatever uses the PartQueue
///     (e.g., transparent or not? what vertex or index buffers should be used?)
///     (2) Instance Type (must be Pod + Zeroable)
///     (3) Uniform Type (must be Pod + Zeroable + Convertable from Instance Type)
pub struct PartQueue<K: Copy, I: Pod + Zeroable, U: Pod + Zeroable + From<I>> {
    /// Unused uniform buffers to be written to with staging belt.
    ///
    /// Also stores the bind group since it's needed with dynamic offsets
    ///     to render with uniform buffers.
    unused_uniform_buffers: VecDeque<(wgpu::Buffer, wgpu::BindGroup)>,
    /// Unused instance buffers to be written to with staging belt.
    unused_instance_buffers: VecDeque<wgpu::Buffer>,

    uniform_belt: StagingBelt,
    instance_belt: StagingBelt,
    /// Hint at whether or not we call submit/recall on uniform staging belt.
    uniform_dirty: bool,
    /// Hint at whether or not we call submit/recall on instancing staging belt.
    instance_dirty: bool,

    ready_uniform_buffers: VecDeque<ReadyBuffer<K>>,
    ready_instance_buffers: VecDeque<ReadyBuffer<K>>,

    _marker: PhantomData<(I, U)>,
}

impl<K: Copy, I: Pod + Zeroable, U: Pod + Zeroable + From<I>> PartQueue<K, I, U> {
    const INSTANCE_SIZE: u64 = std::mem::size_of::<I>() as u64;
    const UNIFORM_SIZE: u64 = std::mem::size_of::<U>() as u64;

    pub fn new(device: &wgpu::Device) -> Self {
        let uniform_count = PartDetails::get().max_uniform_count;
        let layout_size = PartDetails::get().uniform_layout_size;

        let mut part_queue = PartQueue {
            unused_instance_buffers: VecDeque::new(),
            unused_uniform_buffers: VecDeque::new(),
            uniform_belt: StagingBelt::new((layout_size * uniform_count) as u64),
            instance_belt: StagingBelt::new(Self::INSTANCE_SIZE * MAX_INSTANCE_COUNT as u64),
            uniform_dirty: false,
            instance_dirty: false,
            ready_instance_buffers: VecDeque::new(),
            ready_uniform_buffers: VecDeque::new(),
            _marker: PhantomData,
        };
        // Allocate one instance buffer
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

    /// Create a uniform buffer and its bind group
    fn new_uniform_buffer(&self, device: &wgpu::Device) -> (wgpu::Buffer, wgpu::BindGroup) {
        let uniform_count = PartDetails::get().max_uniform_count;
        let layout_size = PartDetails::get().uniform_layout_size;

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Part Uniform Buffer"),
            size: (uniform_count * layout_size) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Part Uniform Bind Group"),
            layout: &PartDetails::get().part_uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new(Self::UNIFORM_SIZE as u64),
                }),
            }],
        });
        (buffer, bind_group)
    }

    /// Create an instance buffer  
    fn new_instance_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        let size = Self::INSTANCE_SIZE * MAX_INSTANCE_COUNT as u64;

        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Part Instance Buffer"),
            size: size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Get an uniform buffer and its bind group by popping unused queue or creating a new one.
    fn get_uniform_buffer(&mut self, device: &wgpu::Device) -> (wgpu::Buffer, wgpu::BindGroup) {
        if let Some(buffer) = self.unused_uniform_buffers.pop_front() {
            buffer
        } else {
            self.new_uniform_buffer(device)
        }
    }

    /// Get an instance buffer by popping unused queue or creating a new one.
    fn get_instance_buffer(&mut self, device: &wgpu::Device) -> wgpu::Buffer {
        if let Some(buffer) = self.unused_instance_buffers.pop_front() {
            buffer
        } else {
            self.new_instance_buffer(device)
        }
    }

    /// Map a contiguous slice of instance data onto a buffer via. a staging belt.
    ///
    /// Given a slice, it'll split its data into multiple buffers that can be retrieved as
    ///     ReadyBuffers.
    ///
    /// Note that the slice should have the same "kind" as if they are rendered
    ///     together using instancing.
    ///
    /// Based off of the slice length, it'll write it onto a uniform or instance buffer.
    ///     Since uniform buffers require padding while instance buffers are packed,
    ///     it will convert the instance format into a uniform format iteratively.
    ///
    /// After calling all map_slices(), finish() should be called.
    pub fn map_slice(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut CommandEncoder,
        key: K,
        instances: &[I],
    ) {
        let max_uniform = PartDetails::get().max_uniform_count;
        if instances.len() <= max_uniform * 4 {
            // Uniform

            // Uniform Buffer Objects work a bit differently than Instance Buffer Objects, they require a padding of 256 bytes,
            //  so we can't naively just copy them here. we will have to get a view then iterate.
            let layout_size = PartDetails::get().uniform_layout_size;

            for chunk in instances.rchunks(max_uniform) {
                let (buffer, bind_group) = self.get_uniform_buffer(device);
                let view_size = NonZero::new((layout_size * chunk.len()) as u64)
                    .expect("View size didn't construct");
                let mut view = self
                    .uniform_belt
                    .write_buffer(encoder, &buffer, 0, view_size, device);

                // re-align to uniform buffer alignment size
                for i in 0..chunk.len() {
                    let uniform = U::from(chunk[i]);
                    view[i * layout_size..i * layout_size + Self::UNIFORM_SIZE as usize]
                        .copy_from_slice(&bytemuck::cast_slice(&[uniform]));
                }
                // Save dynamic offsets to use
                let offsets: Vec<wgpu::DynamicOffset> = (0..chunk.len())
                    .map(|i| (i * layout_size) as wgpu::DynamicOffset)
                    .collect();

                self.ready_uniform_buffers.push_back(ReadyBuffer {
                    key: key,
                    len: chunk.len(),
                    buffer: buffer,
                    bind_and_offsets: Some((bind_group, offsets)),
                });
            }

            self.uniform_dirty = true;
        } else {
            // Instancing
            for chunk in instances.rchunks(MAX_INSTANCE_COUNT) {
                let buffer = self.get_instance_buffer(device);

                let max_len: u64 = Self::INSTANCE_SIZE * chunk.len() as u64;
                let view_size = NonZero::new(max_len).expect("Wut");
                let mut view = self
                    .instance_belt
                    .write_buffer(encoder, &buffer, 0, view_size, device);

                let end = chunk.len() * Self::INSTANCE_SIZE as usize;
                view[0..end].copy_from_slice(bytemuck::cast_slice(chunk));
                self.ready_instance_buffers.push_back(ReadyBuffer {
                    key: key,
                    len: chunk.len(),
                    buffer: buffer,
                    bind_and_offsets: None,
                });
            }
            self.instance_dirty = true;
        }
    }

    /// Same logic as StagingBelt::finish()
    pub fn finish(&mut self) {
        if self.uniform_dirty {
            self.uniform_belt.finish();
        }
        if self.instance_dirty {
            self.instance_belt.finish();
        }
    }

    /// Same logic as StagingBelt::recall()
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

    /// Drains ReadyBuffers stored after map_slice with a callback function on them.
    ///     Buffers move back to being unused.
    pub fn drain_instances<F>(&mut self, mut callback: F)
    where
        F: FnMut(&ReadyBuffer<K>),
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

    /// Drains ReadyBuffers stored after map_slice with a callback function on them.
    ///     Buffers move back to being unused.
    pub fn drain_uniforms<F>(&mut self, mut callback: F)
    where
        F: FnMut(&ReadyBuffer<K>),
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
                        bind_and_offsets.expect("Bind Group should accompany uniform buffer");

                    self.unused_uniform_buffers.push_back((buffer, bind_group));
                }
            }
        }
    }
}
