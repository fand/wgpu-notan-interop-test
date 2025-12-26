use std::sync::Arc;
use wasm_bindgen::JsValue;

pub struct WgpuResources {
    pub surface: wgpu::Surface<'static>,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
}

pub async fn init_wgpu(canvas: web_sys::HtmlCanvasElement) -> Result<WgpuResources, JsValue> {
    let window = web_sys::window().unwrap();
    let dpi = window.device_pixel_ratio();
    let width = (canvas.client_width() as f64 * dpi) as u32;
    let height = (canvas.client_height() as f64 * dpi) as u32;
    canvas.set_width(width);
    canvas.set_height(height);

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::GL,
        ..Default::default()
    });

    let surface_target = wgpu::SurfaceTarget::Canvas(canvas);
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
        .ok_or("Failed to get adapter")?;

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                ..Default::default()
            },
            None,
        )
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let device = Arc::new(device);
    let queue = Arc::new(queue);

    log::info!("wgpu initialized: {:?}", adapter.get_info());

    let surface_caps = surface.get_capabilities(&adapter);
    let format = surface_caps.formats[0];

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width,
        height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    let pipeline = create_pipeline(&device, format);
    let vertex_buffer = create_vertex_buffer(&device);

    Ok(WgpuResources {
        surface,
        device,
        queue,
        pipeline,
        vertex_buffer,
    })
}

fn create_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("wgpu Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("wgpu Pipeline Layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("wgpu Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 8,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                }],
            }],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
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
        multiview: None,
        cache: None,
    })
}

fn create_vertex_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    use wgpu::util::DeviceExt;

    let vertices: &[f32] = &[
        -0.5, -0.5,
         0.5, -0.5,
         0.5,  0.5,
        -0.5, -0.5,
         0.5,  0.5,
        -0.5,  0.5,
    ];

    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("wgpu Vertex Buffer"),
        contents: bytemuck::cast_slice(vertices),
        usage: wgpu::BufferUsages::VERTEX,
    })
}

pub fn draw(resources: &WgpuResources) {
    let frame = resources.surface.get_current_texture().unwrap();
    let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

    let mut encoder = resources.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("wgpu Encoder"),
    });

    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("wgpu Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&resources.pipeline);
        render_pass.set_vertex_buffer(0, resources.vertex_buffer.slice(..));
        render_pass.draw(0..6, 0..1);
    }

    resources.queue.submit(std::iter::once(encoder.finish()));
    frame.present();
}
