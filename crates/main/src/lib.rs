use notan_app::prelude::*;
use notan_app::AppBuilder;
use notan_graphics::color::Color;
use notan_graphics::prelude::*;
use notan_web::WebBackend;
use wasm_bindgen::prelude::*;

fn draw(app: &mut App, gfx: &mut Graphics) {
    // Cycle through colors based on time
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

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let backend = WebBackend::new().map_err(|e| JsValue::from_str(&e))?;

    AppBuilder::new(|| {}, backend)
        .draw(draw)
        .build()
        .map_err(|e| JsValue::from_str(&e))?;

    Ok(())
}
