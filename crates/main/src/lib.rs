use notan_app::prelude::*;
use notan_app::{AppBuilder, WindowConfig};
use notan_graphics::color::Color;
use notan_graphics::prelude::*;
use notan_web::WebBackend;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use std::sync::Arc;

// Notan shaders (GLSL 450 -> SPIR-V -> WebGL2)
//language=glsl
const NOTAN_VERT: ShaderSource = notan_macro::vertex_shader! {
    r#"
    #version 450
    layout(location = 0) in vec2 a_pos;

    void main() {
        gl_Position = vec4(a_pos, 0.0, 1.0);
    }
    "#
};

//language=glsl
const NOTAN_FRAG: ShaderSource = notan_macro::fragment_shader! {
    r#"
    #version 450
    precision mediump float;

    layout(location = 0) out vec4 color;

    layout(set = 0, binding = 0) uniform Locals {
        float u_time;
    };

    void main() {
        float r = sin(u_time * 0.7) * 0.5 + 0.5;
        float g = sin(u_time * 1.1) * 0.5 + 0.5;
        float b = sin(u_time * 1.3) * 0.5 + 0.5;
        color = vec4(r * 0.3, g * 0.3, b * 0.5, 1.0);
    }
    "#
};

struct State {
    // Notan resources
    notan_pipeline: Pipeline,
    notan_vbo: Buffer,
    notan_ubo: Buffer,

    // wgpu resources
    wgpu_surface: wgpu::Surface<'static>,
    wgpu_device: Arc<wgpu::Device>,
    wgpu_queue: Arc<wgpu::Queue>,
    wgpu_pipeline: wgpu::RenderPipeline,
    wgpu_vertex_buffer: wgpu::Buffer,
}

impl AppState for State {}

fn draw(app: &mut App, gfx: &mut Graphics, state: &mut State) {
    let t = app.timer.elapsed_f32();

    // === 1. Draw background with Notan ===
    gfx.set_buffer_data(&state.notan_ubo, &[t]);

    let mut renderer = gfx.create_renderer();
    renderer.begin(Some(ClearOptions::color(Color::BLACK)));
    renderer.set_pipeline(&state.notan_pipeline);
    renderer.bind_buffer(&state.notan_vbo);
    renderer.bind_buffer(&state.notan_ubo);
    renderer.draw(0, 6);
    renderer.end();
    gfx.render(&renderer);

    // === 2. Draw red rectangle with wgpu (preserve existing content) ===
    let frame = state.wgpu_surface.get_current_texture().unwrap();
    let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

    let mut encoder = state.wgpu_device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("wgpu Encoder"),
    });

    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("wgpu Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Keep notan's background
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&state.wgpu_pipeline);
        render_pass.set_vertex_buffer(0, state.wgpu_vertex_buffer.slice(..));
        render_pass.draw(0..6, 0..1);
    }

    state.wgpu_queue.submit(std::iter::once(encoder.finish()));
    frame.present();
}

fn get_canvas() -> Result<web_sys::HtmlCanvasElement, JsValue> {
    let window = web_sys::window().ok_or("No window")?;
    let document = window.document().ok_or("No document")?;
    document
        .get_element_by_id("notan")
        .ok_or("No canvas with id 'notan'")?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| JsValue::from_str("Element is not a canvas"))
}

fn get_webgl2_context(
    canvas: &web_sys::HtmlCanvasElement,
) -> Result<web_sys::WebGl2RenderingContext, JsValue> {
    canvas
        .get_context("webgl2")?
        .ok_or_else(|| JsValue::from_str("Failed to get WebGL2 context"))?
        .dyn_into::<web_sys::WebGl2RenderingContext>()
        .map_err(|_| JsValue::from_str("Failed to cast to WebGl2RenderingContext"))
}

fn create_wgpu_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
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

fn create_wgpu_vertex_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    use wgpu::util::DeviceExt;

    // Red rectangle in center
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

fn setup_notan(gfx: &mut Graphics) -> (Pipeline, Buffer, Buffer) {
    let vertex_info = VertexInfo::new().attr(0, VertexFormat::Float32x2);

    let pipeline = gfx
        .create_pipeline()
        .from(&NOTAN_VERT, &NOTAN_FRAG)
        .with_vertex_info(&vertex_info)
        .build()
        .unwrap();

    // Fullscreen quad
    #[rustfmt::skip]
    let vertices: [f32; 12] = [
        -1.0, -1.0,
         1.0, -1.0,
         1.0,  1.0,
        -1.0, -1.0,
         1.0,  1.0,
        -1.0,  1.0,
    ];

    let vbo = gfx
        .create_vertex_buffer()
        .with_info(&vertex_info)
        .with_data(&vertices)
        .build()
        .unwrap();

    let ubo = gfx
        .create_uniform_buffer(0, "Locals")
        .with_data(&[0.0f32])
        .build()
        .unwrap();

    (pipeline, vbo, ubo)
}

async fn run() -> Result<(), JsValue> {
    let canvas = get_canvas()?;

    // Get proper size with DPI scaling
    let window = web_sys::window().unwrap();
    let dpi = window.device_pixel_ratio();
    let width = (canvas.client_width() as f64 * dpi) as u32;
    let height = (canvas.client_height() as f64 * dpi) as u32;
    canvas.set_width(width);
    canvas.set_height(height);

    // Initialize wgpu
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::GL,
        ..Default::default()
    });

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

    let wgpu_pipeline = create_wgpu_pipeline(&device, format);
    let wgpu_vertex_buffer = create_wgpu_vertex_buffer(&device);

    // Get WebGL2 context and create notan backend
    let webgl2_ctx = get_webgl2_context(&canvas)?;
    let backend =
        WebBackend::with_webgl2_context(webgl2_ctx).map_err(|e| JsValue::from_str(&e))?;

    let win_config = WindowConfig::default().set_app_id("notan");

    let wgpu_surface = surface;
    let wgpu_device = device;
    let wgpu_queue = queue;

    AppBuilder::new(
        move |gfx: &mut Graphics| {
            let (notan_pipeline, notan_vbo, notan_ubo) = setup_notan(gfx);
            State {
                notan_pipeline,
                notan_vbo,
                notan_ubo,
                wgpu_surface,
                wgpu_device,
                wgpu_queue,
                wgpu_pipeline,
                wgpu_vertex_buffer,
            }
        },
        backend,
    )
    .add_config(win_config)
    .draw(draw)
    .build()
    .map_err(|e| JsValue::from_str(&e))?;

    Ok(())
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Info).unwrap();

    wasm_bindgen_futures::spawn_local(async {
        if let Err(e) = run().await {
            log::error!("Error: {:?}", e);
        }
    });
}
