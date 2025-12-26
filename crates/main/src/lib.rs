use notan_app::prelude::*;
use notan_app::{AppBuilder, WindowConfig};
use notan_graphics::color::Color;
use notan_graphics::prelude::*;
use notan_web::WebBackend;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

fn draw(app: &mut App, gfx: &mut Graphics) {
    let t = app.timer.elapsed_f32();
    let r = (t * 0.7).sin() * 0.5 + 0.5;
    let g = (t * 1.1).sin() * 0.5 + 0.5;
    let b = (t * 1.3).sin() * 0.5 + 0.5;
    let clear = ClearOptions::color(Color::new(r, g, b, 1.0));

    let mut renderer = gfx.create_renderer();
    renderer.begin(Some(clear));
    renderer.end();
    gfx.render(&renderer);
}

fn get_webgl2_context(canvas: &web_sys::HtmlCanvasElement) -> Result<web_sys::WebGl2RenderingContext, JsValue> {
    canvas
        .get_context("webgl2")?
        .ok_or_else(|| JsValue::from_str("Failed to get WebGL2 context"))?
        .dyn_into::<web_sys::WebGl2RenderingContext>()
        .map_err(|_| JsValue::from_str("Failed to cast to WebGl2RenderingContext"))
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    // 1. Get canvas element
    let window = web_sys::window().ok_or("No window")?;
    let document = window.document().ok_or("No document")?;
    let canvas = document
        .get_element_by_id("notan")
        .ok_or("No canvas with id 'notan'")?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| "Element is not a canvas")?;

    // 2. Get WebGL2 context from canvas
    let webgl2_ctx = get_webgl2_context(&canvas)?;

    // 3. Pass the context to notan
    let backend = WebBackend::with_webgl2_context(webgl2_ctx)
        .map_err(|e| JsValue::from_str(&e))?;

    // 4. Set app_id to match the canvas id
    let win_config = WindowConfig::default().set_app_id("notan");

    AppBuilder::new(|| {}, backend)
        .add_config(win_config)
        .draw(draw)
        .build()
        .map_err(|e| JsValue::from_str(&e))?;

    Ok(())
}
