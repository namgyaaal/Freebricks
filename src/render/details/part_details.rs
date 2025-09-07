use std::sync::OnceLock;
use wgpu::{BindGroupLayout, PipelineLayout, RenderPipeline};

static PART_DETAILS: OnceLock<PartDetails> = OnceLock::new();

pub fn part_details_init(device: &wgpu::Device) {
    PART_DETAILS
        .set(PartDetails::new(device))
        .expect("part_details_init() already called");
}
pub fn part_details_get() -> &'static PartDetails {
    PART_DETAILS
        .get()
        .expect("part_details_init() hasn't been called before part_details_get()")
}

#[derive(Debug)]
pub struct PartDetails {
    pub bind_layout: wgpu::BindGroupLayout,
    pub pipeline_layout: wgpu::PipelineLayout,
}

impl PartDetails {
    pub fn new(device: &wgpu::Device) -> Self {
        let entries = Vec::from([
            // Stud Texture Array
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            // Stud sampler
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            // Camera
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Light Entry
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ]);

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Part Layout"),
            entries: &entries,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SceneTree Pipeline Layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });

        Self {
            bind_layout: layout,
            pipeline_layout: pipeline_layout,
        }
    }
}
