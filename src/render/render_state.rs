use crate::{common::state::*, render::texture::*};
use anyhow::Result;
use bevy_ecs::prelude::*;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};
use wgpu::{RenderPass, SurfaceTexture};
use winit::window::Window;

#[derive(Resource)]
/// Resource that is used during render stage that keeps the command encoder and render pass as
///     bevy_ecs resources so that they can be used by systems.
pub struct FrameInfo {
    pub command: Option<wgpu::CommandEncoder>,
    pub view: Option<wgpu::TextureView>,
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
}

impl State<RenderState> for RenderState {
    /// Shorthand to make a RenderPassInfo resource and also insert self as a resource.
    fn consume(world: &mut World, state: RenderState) {
        world.insert_resource(state);
        world.insert_resource(FrameInfo {
            command: None,
            view: None,
            pass: None,
            output: None,
        })
    }
}

impl RenderState {
    /// Given a window, create wgpu adapter/device/etc
    pub async fn new(window: Arc<Window>) -> Result<Self> {
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

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::default(),
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
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        Ok(Self {
            window: window,
            surface: surface,
            device: device,
            queue: queue,
            config: config,
            pass: None, // Created on resize()
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

    pub fn begin_frame(&mut self) -> Result<Option<FrameInfo>, wgpu::SurfaceError> {
        self.window.request_redraw();

        if self.pass.is_none() {
            return Ok(None);
        }
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

        let encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        Ok(Some(FrameInfo {
            command: Some(encoder),
            view: Some(view),
            pass: None,
            output: Some(output),
        }))
    }

    /// Start a render pass that can be used to render to
    pub fn begin_pass(state: Res<RenderState>, mut frame: ResMut<FrameInfo>) {
        let frame = frame.deref_mut();

        let encoder = frame
            .command
            .as_mut()
            .expect("begin_pass needs CommandEncoder");

        let view = frame
            .view
            .as_ref()
            .expect("begin_pass expects a TextureView");

        let depth_texture = &state.pass.as_ref().unwrap().depth_texture;

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
            timestamp_writes: None,
        });
        let pass = RenderPass::forget_lifetime(pass);
        frame.pass = Some(pass);
    }

    /// Submit render pass and empty RenderPassInfo
    /// Should only be called after begin_pass()
    pub fn flush(state: Res<RenderState>, mut pass_info: ResMut<FrameInfo>) {
        let encoder = pass_info
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

        state.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        pass_info.view.take();
    }
}
