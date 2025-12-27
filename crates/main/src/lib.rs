mod wgpu;

use notan_app::prelude::*;
use notan_app::{AppBuilder, WindowConfig};
use notan_glow::GlowBackend;
use notan_graphics::color::Color;
use notan_graphics::prelude::*;
use notan_web::WebBackend;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

const WIDTH: u32 = 512;
const HEIGHT: u32 = 512;

// Notan shaders for rendering textures (GLSL 300 ES for WebGL2)
const TEXTURE_VERT_SRC: &[u8] = br#"#version 300 es
in vec2 a_pos;
in vec2 a_uv;
out vec2 v_uv;

void main() {
    v_uv = a_uv;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
"#;

const TEXTURE_FRAG_SRC: &[u8] = br#"#version 300 es
precision mediump float;

in vec2 v_uv;
out vec4 color;

uniform sampler2D u_texture;

void main() {
    color = texture(u_texture, v_uv);
}
"#;

const TEXTURE_VERT: ShaderSource = ShaderSource {
    sources: &[("webgl2", TEXTURE_VERT_SRC)],
};

const TEXTURE_FRAG: ShaderSource = ShaderSource {
    sources: &[("webgl2", TEXTURE_FRAG_SRC)],
};

struct State {
    // Textures
    ferris: Texture,
    texture1: RenderTexture,
    texture2: RenderTexture,

    // Notan resources
    texture_pipeline: Pipeline,
    quad_vbo: Buffer,

    // wgpu processor
    wgpu_processor: wgpu::WgpuProcessor,
}

impl AppState for State {}

fn draw(_app: &mut App, gfx: &mut Graphics, state: &mut State) {
    // === 1. Render ferris to texture1 using Notan ===
    {
        let mut renderer = gfx.create_renderer();
        renderer.begin(Some(ClearOptions::color(Color::TRANSPARENT)));
        renderer.set_pipeline(&state.texture_pipeline);
        renderer.bind_texture(0, &state.ferris);
        renderer.bind_buffer(&state.quad_vbo);
        renderer.draw(0, 6);
        renderer.end();
        gfx.render_to(&state.texture1, &renderer);
    }

    // === 2. Process texture1 -> texture2 with RGB inversion (wgpu) ===
    {
        let backend = gfx.device.downcast_backend::<GlowBackend>().unwrap();

        // Flush Notan's WebGL commands before wgpu reads
        backend.flush();

        let input_raw = backend
            .get_raw_texture(state.texture1.texture().id())
            .expect("Failed to get texture1 raw handle");
        let output_raw = backend
            .get_raw_texture(state.texture2.texture().id())
            .expect("Failed to get texture2 raw handle");

        // Initialize textures once (cached internally)
        state
            .wgpu_processor
            .init_textures(input_raw, output_raw, WIDTH, HEIGHT);
        state.wgpu_processor.invert();

        // Reset wgpu's sampler bindings so Notan can render correctly
        backend.unbind_samplers();
    }

    // === 3. Render texture2 to screen using Notan ===
    {
        let mut renderer = gfx.create_renderer();
        renderer.begin(Some(ClearOptions::color(Color::BLACK)));
        renderer.set_pipeline(&state.texture_pipeline);
        renderer.bind_texture(0, state.texture2.texture());
        renderer.bind_buffer(&state.quad_vbo);
        renderer.draw(0, 6);
        renderer.end();
        gfx.render(&renderer);
    }
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

fn setup(gfx: &mut Graphics, wgpu_processor: wgpu::WgpuProcessor) -> State {
    // Load ferris.png
    let ferris_bytes = include_bytes!("../ferris.png");
    let ferris = gfx
        .create_texture()
        .from_image(ferris_bytes)
        .build()
        .expect("Failed to load ferris.png");

    // Create render textures
    let texture1 = gfx
        .create_render_texture(WIDTH, HEIGHT)
        .build()
        .expect("Failed to create texture1");

    let texture2 = gfx
        .create_render_texture(WIDTH, HEIGHT)
        .build()
        .expect("Failed to create texture2");

    // Create texture rendering pipeline
    let vertex_info = VertexInfo::new()
        .attr(0, VertexFormat::Float32x2) // position
        .attr(1, VertexFormat::Float32x2); // uv

    let texture_pipeline = gfx
        .create_pipeline()
        .from(&TEXTURE_VERT, &TEXTURE_FRAG)
        .with_vertex_info(&vertex_info)
        .with_texture_location(0, "u_texture")
        .build()
        .expect("Failed to create texture pipeline");

    // Fullscreen quad with UVs
    #[rustfmt::skip]
    let vertices: [f32; 24] = [
        // pos      // uv
        -1.0, -1.0, 0.0, 0.0,
         1.0, -1.0, 1.0, 0.0,
         1.0,  1.0, 1.0, 1.0,
        -1.0, -1.0, 0.0, 0.0,
         1.0,  1.0, 1.0, 1.0,
        -1.0,  1.0, 0.0, 1.0,
    ];

    let quad_vbo = gfx
        .create_vertex_buffer()
        .with_info(&vertex_info)
        .with_data(&vertices)
        .build()
        .expect("Failed to create quad VBO");

    State {
        ferris,
        texture1,
        texture2,
        texture_pipeline,
        quad_vbo,
        wgpu_processor,
    }
}

async fn run() -> Result<(), JsValue> {
    let canvas = get_canvas()?;

    // Set canvas size
    let window = web_sys::window().unwrap();
    let dpi = window.device_pixel_ratio();
    let width = (canvas.client_width() as f64 * dpi) as u32;
    let height = (canvas.client_height() as f64 * dpi) as u32;
    canvas.set_width(width);
    canvas.set_height(height);

    // Initialize wgpu processor first (before Notan takes the context)
    let wgpu_processor = wgpu::WgpuProcessor::new(&canvas).await?;

    // Get WebGL2 context and create notan backend
    let webgl2_ctx = get_webgl2_context(&canvas)?;
    let backend = WebBackend::with_webgl2_context(webgl2_ctx).map_err(|e| JsValue::from_str(&e))?;

    let win_config = WindowConfig::default().set_app_id("notan");

    AppBuilder::new(
        move |gfx: &mut Graphics| setup(gfx, wgpu_processor),
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
