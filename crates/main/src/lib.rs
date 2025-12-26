mod wgpu;

use notan_app::prelude::*;
use notan_app::{AppBuilder, WindowConfig};
use notan_graphics::color::Color;
use notan_graphics::prelude::*;
use notan_web::WebBackend;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

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
    wgpu: wgpu::WgpuResources,
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

    // === 2. Draw red rectangle with wgpu ===
    wgpu::draw(&state.wgpu);
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

    // Initialize wgpu
    let wgpu_resources = wgpu::init_wgpu(canvas.clone()).await?;

    // Get WebGL2 context and create notan backend
    let webgl2_ctx = get_webgl2_context(&canvas)?;
    let backend =
        WebBackend::with_webgl2_context(webgl2_ctx).map_err(|e| JsValue::from_str(&e))?;

    let win_config = WindowConfig::default().set_app_id("notan");

    AppBuilder::new(
        move |gfx: &mut Graphics| {
            let (notan_pipeline, notan_vbo, notan_ubo) = setup_notan(gfx);
            State {
                notan_pipeline,
                notan_vbo,
                notan_ubo,
                wgpu: wgpu_resources,
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
