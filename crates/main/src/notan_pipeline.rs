use notan_app::Graphics;
use notan_graphics::prelude::*;

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

pub fn create_texture_pipeline(gfx: &mut Graphics) -> (Pipeline, Buffer, VertexInfo) {
    let vertex_info = VertexInfo::new()
        .attr(0, VertexFormat::Float32x2) // position
        .attr(1, VertexFormat::Float32x2); // uv

    let pipeline = gfx
        .create_pipeline()
        .from(&TEXTURE_VERT, &TEXTURE_FRAG)
        .with_vertex_info(&vertex_info)
        .with_texture_location(0, "u_texture")
        .with_color_blend(BlendMode::NORMAL)
        .with_alpha_blend(BlendMode::NONE)
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

    (pipeline, quad_vbo, vertex_info)
}
