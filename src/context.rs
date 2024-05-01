use std::sync::Arc;
use librashader::presets::ShaderPreset;
use librashader::runtime::wgpu::FilterChain;
use librashader::runtime::Viewport;
use wgpu::PresentMode;
use winit::dpi::PhysicalSize;
use winit::window::Window;
use crate::components::prelude::ppu::{SCREEN_H, SCREEN_W};

pub struct Context {
    pub surface: wgpu::Surface<'static>,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    render_output: Arc<wgpu::Texture>,
    pub config: wgpu::SurfaceConfiguration,
    pub size: PhysicalSize<u32>,
    window: Arc<Window>,
    chain: FilterChain,
    frame_count: usize,
}

impl Context {
    pub async fn new(window: Arc<Window>, shader_path: String) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::default();

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER,
                    required_limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats[0];

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let preset = ShaderPreset::try_parse(shader_path).unwrap();
        let chain = FilterChain::load_from_preset(
            preset,
            Arc::clone(&device),
            Arc::clone(&queue),
            None,
        ).unwrap();

        let render_output = Arc::new(device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rendertexture"),
            size: wgpu::Extent3d {
                width: SCREEN_W as u32,
                height: SCREEN_H as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        }));

        Self {
            surface,
            device,
            queue,
            render_output,
            config,
            size,
            window,
            chain,
            frame_count: 0,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }

        self.window.request_redraw();
    }

    pub fn update(&mut self, rgba: &[u8]) {
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &self.render_output,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * SCREEN_W as u32),
                rows_per_image: Some(SCREEN_H as u32),
            },
            wgpu::Extent3d {
                width: SCREEN_W as u32,
                height: SCREEN_H as u32,
                depth_or_array_layers: 1,
            },
        );
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;

        let filter_output = Arc::new(self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("filteroutput"),
            size: output.texture.size(),
            mip_level_count: output.texture.mip_level_count(),
            sample_count: output.texture.sample_count(),
            dimension: output.texture.dimension(),
            format: output.texture.format(),
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[output.texture.format()],
        }));

        let filter_view = filter_output.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.chain
            .frame(
                Arc::clone(&self.render_output),
                &Viewport {
                    x: 0.0,
                    y: 0.0,
                    mvp: None,
                    output: librashader::runtime::wgpu::WgpuOutputView::new_from_raw(
                        &filter_view,
                        filter_output.size().into(),
                        filter_output.format(),
                    ),
                },
                &mut encoder,
                self.frame_count,
                None,
            ).expect("Failed to draw frame!");

        encoder.copy_texture_to_texture(
            filter_output.as_image_copy(),
            output.texture.as_image_copy(),
            output.texture.size(),
        );

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        self.frame_count += 1;
        Ok(())
    }
}
