// https://github.com/gfx-rs/wgpu/blob/trunk/examples/features/src/timestamp_queries/mod.rs


pub struct Queries {
    pub set: wgpu::QuerySet,
    pub resolve_buffer: wgpu::Buffer,
    pub destination_buffer: wgpu::Buffer,
}

pub struct QueryResults {
    pub render_start_end_timestamps: [u64; 2]
}

impl QueryResults {
    // Queries:
    // * render start
    // * render end
    pub const NUM_QUERIES: u64 = 2;

    #[expect(
        clippy::redundant_closure,
        reason = "false positive for `get_next_slot`, which needs to be used by reference"
    )]
    pub fn from_raw_results(timestamps: Vec<u64>) -> Self {
        assert_eq!(timestamps.len(), Self::NUM_QUERIES as usize);

        let mut next_slot = 0;
        let mut get_next_slot = || {
            let slot = timestamps[next_slot];
            next_slot += 1;
            slot
        };


        let render_start_end_timestamps = [get_next_slot(), get_next_slot()];

        QueryResults {
            render_start_end_timestamps
        }
    }

    pub fn print(&self, queue: &wgpu::Queue) {
        let period = queue.get_timestamp_period();
        let elapsed_ns = |start, end: u64| end.wrapping_sub(start) as f64 * period as f64;

        println!(
            "Elapsed time render pass: {:.8} μs",
            elapsed_ns(
                self.render_start_end_timestamps[0],
                self.render_start_end_timestamps[1]
            ) / 1_000_000.0
        );

    }
}

impl Queries {
    pub fn new(device: &wgpu::Device) -> Self {
        Queries {
            set: device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("Timestamp query set"),
                count: 2,
                ty: wgpu::QueryType::Timestamp,
            }),
            resolve_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("query resolve buffer"),
                size: 16,
                usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::QUERY_RESOLVE,
                mapped_at_creation: false,
            }),
            destination_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("query dest buffer"),
                size: 16,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            })
        }
    }

    pub fn resolve(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.resolve_query_set(
            &self.set,
            0..2,
            &self.resolve_buffer,
            0,
        );
        encoder.copy_buffer_to_buffer(
            &self.resolve_buffer,
            0,
            &self.destination_buffer,
            0,
            self.resolve_buffer.size(),
        );
    }

    pub fn clear_buffers(&self, encoder: &mut wgpu::CommandEncoder) {
        // Clear the resolve buffer with zeros
        encoder.clear_buffer(&self.resolve_buffer, 0, None);
        encoder.clear_buffer(&self.destination_buffer, 0, None);
    }


    pub fn get_timestamp(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Option<f32> {
        self.destination_buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, |_| ());
        //device.poll(wgpu::PollType::wait()).unwrap();
        device.poll(wgpu::PollType::Wait).unwrap();

        let timestamps: Vec<u64> = {
            let timestamp_view = self
                .destination_buffer
                .slice(..)
                .get_mapped_range();
            bytemuck::cast_slice(&timestamp_view).to_vec()
        };

        self.destination_buffer.unmap();
        
        // Non-monotonic timestamps on OSX are common 
        if timestamps[1] <= timestamps[0] {
            return None; 
        }

        let period = queue.get_timestamp_period();

        let dur = timestamps[1].wrapping_sub(timestamps[0]) as f32 * period / 1_000_000.0; 

        Some(dur)
    }
}