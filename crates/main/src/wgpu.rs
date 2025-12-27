use std::cell::RefCell;
use std::sync::Arc;
use wasm_bindgen::JsValue;
use wgpu::hal;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct TimeUniform {
    time: f32,
    _pad: [f32; 3], // Pad to 16 bytes for WebGL
}

pub struct WgpuProcessor {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub format: wgpu::TextureFormat,
    // wgpu pipeline resources
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
    bind_group_layout: wgpu::BindGroupLayout,
    time_buffer: wgpu::Buffer,
    // Cached textures (to avoid recreating/dropping each frame)
    cached_input: RefCell<Option<wgpu::Texture>>,
    cached_output: RefCell<Option<wgpu::Texture>>,
    cached_bind_group: RefCell<Option<wgpu::BindGroup>>,
}

impl WgpuProcessor {
    pub async fn new(canvas: &web_sys::HtmlCanvasElement) -> Result<Self, JsValue> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        // Create a temporary surface just to get adapter
        let surface_target = wgpu::SurfaceTarget::Canvas(canvas.clone());
        let surface = instance
            .create_surface(surface_target)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                ..Default::default()
            })
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let device: Arc<wgpu::Device> = Arc::new(device);
        let queue: Arc<wgpu::Queue> = Arc::new(queue);

        log::info!("wgpu processor initialized: {:?}", adapter.get_info());

        let surface_caps = surface.get_capabilities(&adapter);
        let format = surface_caps.formats[0];

        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Hue Rotate Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("huerot.wgsl").into()),
        });

        // Create sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Hue Rotate Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        // Create time uniform buffer
        let time_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Time Buffer"),
            size: std::mem::size_of::<TimeUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Hue Rotate Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Hue Rotate Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            ..Default::default()
        });

        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Hue Rotate Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Ok(Self {
            device,
            queue,
            format,
            pipeline,
            sampler,
            bind_group_layout,
            time_buffer,
            cached_input: RefCell::new(None),
            cached_output: RefCell::new(None),
            cached_bind_group: RefCell::new(None),
        })
    }

    /// Initialize cached textures (call once after getting raw texture handles)
    pub fn init_textures(
        &self,
        input_raw: web_sys::WebGlTexture,
        output_raw: web_sys::WebGlTexture,
        width: u32,
        height: u32,
    ) {
        if self.cached_input.borrow().is_some() {
            return; // Already initialized
        }

        let input_texture = self.wrap_raw_texture(input_raw, width, height, false);
        let output_texture = self.wrap_raw_texture(output_raw, width, height, true);

        let input_view = input_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Hue Rotate Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&input_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.time_buffer.as_entire_binding(),
                },
            ],
        });

        *self.cached_input.borrow_mut() = Some(input_texture);
        *self.cached_output.borrow_mut() = Some(output_texture);
        *self.cached_bind_group.borrow_mut() = Some(bind_group);
    }

    /// Process input texture with hue rotation using wgpu
    pub fn process(&self, time: f32) {
        // Update time uniform
        self.queue.write_buffer(&self.time_buffer, 0, bytemuck::cast_slice(&[TimeUniform { time, _pad: [0.0; 3] }]));

        let output_ref = self.cached_output.borrow();
        let bind_group_ref = self.cached_bind_group.borrow();

        let output_texture = output_ref.as_ref().expect("Textures not initialized");
        let bind_group = bind_group_ref.as_ref().expect("Bind group not initialized");

        let output_view = output_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Hue Rotate Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Hue Rotate Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Wrap a raw WebGL texture as a wgpu texture
    fn wrap_raw_texture(
        &self,
        raw_texture: web_sys::WebGlTexture,
        width: u32,
        height: u32,
        is_render_target: bool,
    ) -> wgpu::Texture {
        let desc = wgpu::TextureDescriptor {
            label: Some("Wrapped WebGL Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: if is_render_target {
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING
            } else {
                wgpu::TextureUsages::TEXTURE_BINDING
            },
            view_formats: &[],
        };

        // Convert TextureDescriptor to hal format
        let hal_desc = hal::TextureDescriptor {
            label: desc.label,
            size: desc.size,
            mip_level_count: desc.mip_level_count,
            sample_count: desc.sample_count,
            dimension: desc.dimension,
            format: desc.format,
            usage: wgpu::TextureUses::from_bits(desc.usage.bits() as u16).unwrap_or(wgpu::TextureUses::empty()),
            memory_flags: hal::MemoryFlags::empty(),
            view_formats: vec![],
        };

        let format_desc = hal::gles::TextureFormatDesc::rgba8_srgb();

        // Access hal device to get glow context and register raw texture
        unsafe {
            let hal_device = self.device.as_hal::<hal::api::Gles>().expect("Failed to access hal device");
            let gl = hal_device.glow_context();
            let hal_texture = hal::gles::Texture::from_raw_webgl(gl, raw_texture, &hal_desc, format_desc);
            self.device.create_texture_from_hal::<hal::api::Gles>(hal_texture, &desc)
        }
    }
}
