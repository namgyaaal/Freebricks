use std::{borrow::Cow, sync::OnceLock};
use wgpu::SurfaceConfiguration;

use crate::render::{
    parts::part_formats::{PartInstance, PartUniform, PartVertex},
    texture::Texture,
};

static PART_DETAILS: OnceLock<PartDetails> = OnceLock::new();

/// Global rendering information and data about parts contingent on having a device.
///
/// Vaguely:
///     (1) Bind Group layouts
///     (2) Pipeline layouts and pipelines
///     (3) Size limits
///
#[derive(Debug)]
pub struct PartDetails {
    pub part_bind_layout: wgpu::BindGroupLayout,
    pub part_uniform_layout: wgpu::BindGroupLayout,
    pub instancing_pipeline_layout: wgpu::PipelineLayout,
    pub instancing_pipeline: wgpu::RenderPipeline,
    pub instancing_pipeline_transparent: wgpu::RenderPipeline,
    pub uniform_pipeline_layout: wgpu::PipelineLayout,
    pub uniform_pipeline: wgpu::RenderPipeline,
    pub uniform_pipeline_transparent: wgpu::RenderPipeline,
    /// Size of PartUniform per-layout. Depends on min_uniform_buffer_offset_alignment
    pub uniform_layout_size: usize,
    /// Number of uniforms to fit in a uniform buffer.
    /// Depends on min_uniform_buffer_offset_alignment and max_uniform_buffer_binding_size
    pub max_uniform_count: usize,
}

impl PartDetails {
    pub fn init(device: &wgpu::Device, config: &SurfaceConfiguration, shader_source: Cow<str>) {
        PART_DETAILS
            .set(PartDetails::new(device, config, shader_source))
            .expect("PartDetails::init() is already called");
    }

    pub fn get() -> &'static PartDetails {
        PART_DETAILS
            .get()
            .expect("PartDetails::init() hasn't been called before PartDetails::get()")
    }

    pub fn new(
        device: &wgpu::Device,
        config: &SurfaceConfiguration,
        shader_source: Cow<str>,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Part Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source),
        });

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

        let part_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Part Layout"),
            entries: &entries,
        });

        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Part Uniform Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Pipeline Layouts

        let instancing_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Part Instancing Pipeline Layout"),
                bind_group_layouts: &[&part_layout],
                push_constant_ranges: &[],
            });

        let uniform_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Part Uniform Pipeline Layout"),
                bind_group_layouts: &[&part_layout, &uniform_layout],
                push_constant_ranges: &[],
            });

        // Render Pipeline Layouts

        // We have a lot of pipelines that have repeated states, save them here

        let shared_opaque_fragment_state = wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: config.format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        };

        let shared_transparent_fragment_state = wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: config.format,
                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        };

        let shared_primitive_state = wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Cw,
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        };

        let shared_multisample_state = wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        };

        let shared_opaque_depth_state = wgpu::DepthStencilState {
            format: Texture::DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        };

        let shared_transparent_depth_state = wgpu::DepthStencilState {
            format: Texture::DEPTH_FORMAT,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        };

        // Define the pipelines here

        let instancing_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Part Instancing Render Pipeline"),
                layout: Some(&instancing_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main_instanced"),
                    buffers: &[PartVertex::desc(), PartInstance::desc_instancing()],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(shared_opaque_fragment_state.clone()),
                primitive: shared_primitive_state,
                depth_stencil: Some(shared_opaque_depth_state.clone()),
                multisample: shared_multisample_state,
                multiview: None,
                cache: None,
            });

        let instancing_render_pipeline_transparent =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Part Instancing Render Pipeline"),
                layout: Some(&instancing_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main_instanced"),
                    buffers: &[PartVertex::desc(), PartInstance::desc_instancing()],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(shared_transparent_fragment_state.clone()),
                primitive: shared_primitive_state,
                depth_stencil: Some(shared_transparent_depth_state.clone()),
                multisample: shared_multisample_state,
                multiview: None,
                cache: None,
            });

        let uniform_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Part Uniform Render Pipeline"),
                layout: Some(&uniform_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main_uniform"),
                    buffers: &[PartVertex::desc()],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(shared_opaque_fragment_state.clone()),
                primitive: shared_primitive_state,
                depth_stencil: Some(shared_opaque_depth_state.clone()),
                multisample: shared_multisample_state,
                multiview: None,
                cache: None,
            });

        let uniform_render_pipeline_transparent =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Part Uniform Render Pipeline"),
                layout: Some(&uniform_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main_uniform"),
                    buffers: &[PartVertex::desc()],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(shared_transparent_fragment_state.clone()),
                primitive: shared_primitive_state,
                depth_stencil: Some(shared_transparent_depth_state.clone()),
                multisample: shared_multisample_state,
                multiview: None,
                cache: None,
            });

        let layout_step = device.limits().min_uniform_buffer_offset_alignment as usize;

        let mut uniform_size = std::mem::size_of::<PartUniform>();
        uniform_size = uniform_size - (uniform_size % layout_step) + layout_step;
        let uniform_count = device.limits().max_uniform_buffer_binding_size as usize / uniform_size;

        Self {
            part_bind_layout: part_layout,
            part_uniform_layout: uniform_layout,
            instancing_pipeline_layout,
            instancing_pipeline: instancing_render_pipeline,
            instancing_pipeline_transparent: instancing_render_pipeline_transparent,
            uniform_pipeline_layout: uniform_pipeline_layout,
            uniform_pipeline: uniform_render_pipeline,
            uniform_pipeline_transparent: uniform_render_pipeline_transparent,
            uniform_layout_size: uniform_size,
            max_uniform_count: uniform_count,
        }
    }

    pub fn get_pipeline(&self, uniform: bool, opaque: bool) -> &wgpu::RenderPipeline {
        match (uniform, opaque) {
            (true, true) => &self.uniform_pipeline,
            (true, false) => &self.uniform_pipeline_transparent,
            (false, true) => &self.instancing_pipeline,
            (false, false) => &self.instancing_pipeline_transparent,
        }
    }
}
