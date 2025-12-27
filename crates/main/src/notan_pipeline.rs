use notan_app::Graphics;
use notan_graphics::prelude::*;

// Static vertex shader (no animation)
const STATIC_VERT_SRC: &[u8] = br#"#version 300 es
in vec2 a_pos;
in vec2 a_uv;
out vec2 v_uv;

void main() {
    v_uv = a_uv;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
"#;

// Animated vertex shader
const ANIMATED_VERT_SRC: &[u8] = br#"#version 300 es
precision mediump float;

in vec2 a_pos;
in vec2 a_uv;
out vec2 v_uv;

layout(std140) uniform Locals {
    float u_time;
    float u_flip_y;
};

void main() {
    v_uv = a_uv;

    float angle = u_time * 6.28318530718 / 5.0;
    vec2 d = vec2(cos(angle), sin(angle)) * 0.1;

    gl_Position = vec4(a_pos + d, 0.0, 1.0);
}
"#;

const TEXTURE_FRAG_SRC: &[u8] = br#"#version 300 es
precision mediump float;

in vec2 v_uv;
out vec4 color;

uniform sampler2D u_texture;

layout(std140) uniform Locals {
    float u_time;
    float u_flip_y;
};

void main() {
    vec2 uv = u_flip_y > 0.5 ? vec2(v_uv.x, 1.0 - v_uv.y) : v_uv;
    color = texture(u_texture, uv);
}
"#;

const STATIC_VERT: ShaderSource = ShaderSource {
    sources: &[("webgl2", STATIC_VERT_SRC)],
};

const ANIMATED_VERT: ShaderSource = ShaderSource {
    sources: &[("webgl2", ANIMATED_VERT_SRC)],
};

const TEXTURE_FRAG: ShaderSource = ShaderSource {
    sources: &[("webgl2", TEXTURE_FRAG_SRC)],
};

pub struct NotanPipelines {
    pub static_pipeline: Pipeline,   // Step1用
    pub animated_pipeline: Pipeline, // Step3用
    pub quad_vbo: Buffer,
    pub ubo: Buffer,
}

pub fn create_pipelines(gfx: &mut Graphics) -> NotanPipelines {
    let vertex_info = VertexInfo::new()
        .attr(0, VertexFormat::Float32x2) // position
        .attr(1, VertexFormat::Float32x2); // uv

    // Static pipeline (step1)
    let static_pipeline = gfx
        .create_pipeline()
        .from(&STATIC_VERT, &TEXTURE_FRAG)
        .with_vertex_info(&vertex_info)
        .with_texture_location(0, "u_texture")
        .with_color_blend(BlendMode::NORMAL)
        .with_alpha_blend(BlendMode::NONE)
        .build()
        .expect("Failed to create static pipeline");

    // Animated pipeline (step3)
    let animated_pipeline = gfx
        .create_pipeline()
        .from(&ANIMATED_VERT, &TEXTURE_FRAG)
        .with_vertex_info(&vertex_info)
        .with_texture_location(0, "u_texture")
        .with_color_blend(BlendMode::NORMAL)
        .with_alpha_blend(BlendMode::NONE)
        .build()
        .expect("Failed to create animated pipeline");

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

    // UBO: [time: f32, flip_y: i32]
    let ubo = gfx
        .create_uniform_buffer(0, "Locals")
        .with_data(&[0.0f32, 0.0f32]) // time, flip_y (as f32 for alignment)
        .build()
        .expect("Failed to create UBO");

    NotanPipelines {
        static_pipeline,
        animated_pipeline,
        quad_vbo,
        ubo,
    }
}
