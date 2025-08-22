use crate::{
    common::state::*,
    render::{queries::Queries, texture::*},
};
use anyhow::Result;
use bevy_ecs::prelude::*;
use enumflags2::{BitFlags, bitflags};
use std::sync::Arc;
use wgpu::{RenderPass, SurfaceTexture};
use winit::window::Window;

#[bitflags]
#[repr(u32)]
#[derive(Copy, Clone, Debug)]
pub enum RenderOptions {
    RenderTimestamps = 0x01,
}

#[derive(Resource)]
/// Resource that is used during render stage that keeps the command encoder and render pass as
///     bevy_ecs resources so that they can be used by systems.
pub struct RenderPassInfo {
    pub command: Option<wgpu::CommandEncoder>,
    // Lifetime is dropped in this context.
    pub pass: Option<wgpu::RenderPass<'static>>,
    pub output: Option<SurfaceTexture>,
}
// Render Pass Data is every pass instead of existing for the pass (renderpassinfo)
pub struct RenderPassData {
    depth_texture: Texture, // Fill up with more when needed.
}

#[derive(Resource)]
pub struct RenderState {
    pub window: Arc<Window>,

    pub surface: wgpu::Surface<'static>,

    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub pass: Option<RenderPassData>,

    pub options: BitFlags<RenderOptions>,
    pub queries: Option<Queries>,
}

impl State<RenderState> for RenderState {
    /// Shorthand to make a RenderPassInfo resource and also insert self as a resource.
    fn consume(world: &mut World, state: RenderState) {
        world.insert_resource(state);
        world.insert_resource(RenderPassInfo {
            command: None,
            pass: None,
            output: None,
        })
    }
}

impl RenderState {
    /// Given a window, create wgpu adapter/device/etc
    pub async fn new(window: Arc<Window>, options: BitFlags<RenderOptions>) -> Result<Self> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone())?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        // Check for GPU profiling.
        let features = {
            if options.contains(RenderOptions::RenderTimestamps) {
                wgpu::Features::TIMESTAMP_QUERY
            } else {
                wgpu::Features::empty()
            }
        };

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: features,
                required_limits: wgpu::Limits::default(),
                //required_limits : wgpu::Limits::downlevel_defaults(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let queries = {
            if options.contains(RenderOptions::RenderTimestamps) {
                Some(Queries::new(&device))
            } else {
                None
            }
        };

        Ok(Self {
            window: window,
            surface: surface,
            device: device,
            queue: queue,
            config: config,
            pass: None, // Created on resize()
            options: options,
            queries: queries,
        })
    }

    /// Resize window
    /// also used as a hint to start the surface for wgpu
    pub fn resize(&mut self, _width: u32, _height: u32) {
        if _width == 0 || _height == 0 {
            return;
        }

        self.config.width = _width;
        self.config.height = _height;

        let depth_texture =
            Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
        self.surface.configure(&self.device, &self.config);

        // Need to refactor when more is added
        match self.pass {
            _ => self.pass = Some(RenderPassData { depth_texture }),
        }
    }

    /// Start a render pass that can be used to render to
    pub fn begin_pass(&mut self) -> Result<Option<RenderPassInfo>, wgpu::SurfaceError> {
        self.window.request_redraw();

        if self.pass.is_none() {
            return Ok(None);
        }
        // Borrow depth texture for the render pass
        let depth_texture = &self.pass.as_mut().unwrap().depth_texture;

        let possible_output = self.surface.get_current_texture();
        // If error is recoverable
        match possible_output {
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                let size = self.window.inner_size();
                self.resize(size.width, size.height);
                return Ok(None);
            }
            Err(e) => {
                return Err(e);
            }
            Ok(_) => {
                //continue
            }
        }
        let output = possible_output.unwrap();

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Timestamp feature
        let timestamp_writes = {
            if self.options.contains(RenderOptions::RenderTimestamps)
                && let Some(queries) = &self.queries
            {
                Some(wgpu::RenderPassTimestampWrites {
                    query_set: &queries.set,
                    beginning_of_pass_write_index: Some(0),
                    end_of_pass_write_index: Some(1),
                })
            } else {
                None
            }
        };

        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Brick Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: timestamp_writes,
        });
        let pass = RenderPass::forget_lifetime(pass);

        Ok(Some(RenderPassInfo {
            command: Some(encoder),
            pass: Some(pass),
            output: Some(output),
        }))
    }

    /// Submit render pass and empty RenderPassInfo
    /// Should only be called after begin_pass()
    pub fn flush(state: Res<RenderState>, mut pass_info: ResMut<RenderPassInfo>) {
        let mut encoder = pass_info
            .command
            .take()
            .expect("RenderState::flush(), expected encoder");

        let output = pass_info
            .output
            .take()
            .expect("RenderState::flush(), expected output");

        // Simulate dropping it here since we called RenderPass::forget_lifetime() in RenderState:begin_pass()
        {
            let _pass = pass_info
                .pass
                .take()
                .expect("RenderState::flush(), expected render pass");
        }

        if let Some(queries) = &state.queries {
            queries.resolve(&mut encoder);
        }
        state.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        if let Some(queries) = &state.queries {
            match queries.get_timestamp(&state.device, &state.queue) {
                None => {}
                Some(_timestamp) => {
                    //tracing::info!("GPU Step: {:.3}", _timestamp);
                }
            }
        }
    }
}
