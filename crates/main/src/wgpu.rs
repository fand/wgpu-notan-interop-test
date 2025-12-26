use std::sync::Arc;
use wasm_bindgen::JsValue;
use notan_glow::TextureKey;

pub struct WgpuProcessor {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub format: wgpu::TextureFormat,
    // glow shader program (reused across frames)
    invert_program: Option<glow::Program>,
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

        log::info!("wgpu processor initialized: {:?}", adapter.get_info());

        let surface_caps = surface.get_capabilities(&adapter);
        let format = surface_caps.formats[0];

        Ok(Self {
            device,
            queue,
            format,
            invert_program: None,
        })
    }

    /// Initialize the glow shader program (must be called with gl context)
    pub fn init_shader(&mut self, gl: &glow::Context) {
        if self.invert_program.is_none() {
            self.invert_program = Some(unsafe { create_invert_program(gl) });
        }
    }

    /// Process input texture with RGB inversion, render to output texture
    /// Uses glow (same WebGL2 context) for actual rendering
    pub fn invert(
        &mut self,
        gl: &glow::Context,
        input_handle: TextureKey,
        output_handle: TextureKey,
        width: u32,
        height: u32,
    ) {
        self.init_shader(gl);

        use glow::HasContext;

        unsafe {

            // Create framebuffer and attach output texture
            let fbo = gl.create_framebuffer().unwrap();
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(output_handle),
                0,
            );

            // Set viewport
            gl.viewport(0, 0, width as i32, height as i32);

            // Use our shader program
            let program = self.invert_program.unwrap();
            gl.use_program(Some(program));

            // Bind input texture
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(input_handle));
            let loc = gl.get_uniform_location(program, "u_texture");
            gl.uniform_1_i32(loc.as_ref(), 0);

            // Create VAO and VBO for fullscreen quad
            let vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(vao));

            let vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));

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

            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&vertices),
                glow::STATIC_DRAW,
            );

            // Position attribute
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 16, 0);

            // UV attribute
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, 16, 8);

            // Draw
            gl.draw_arrays(glow::TRIANGLES, 0, 6);

            // Cleanup temporary resources
            gl.delete_buffer(vbo);
            gl.delete_vertex_array(vao);

            // Restore state
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.delete_framebuffer(fbo);
        }
    }
}

impl Drop for WgpuProcessor {
    fn drop(&mut self) {
        // Note: We can't clean up the glow program here since we don't have gl context
        // In a real app, you'd want to handle this properly
    }
}

unsafe fn create_invert_program(gl: &glow::Context) -> glow::Program {
    use glow::HasContext;

    let vert_src = r#"#version 300 es
        layout(location = 0) in vec2 a_pos;
        layout(location = 1) in vec2 a_uv;
        out vec2 v_uv;
        void main() {
            v_uv = a_uv;
            gl_Position = vec4(a_pos, 0.0, 1.0);
        }
    "#;

    let frag_src = r#"#version 300 es
        precision mediump float;
        in vec2 v_uv;
        out vec4 color;
        uniform sampler2D u_texture;
        void main() {
            vec4 c = texture(u_texture, v_uv);
            color = vec4(1.0 - c.rgb, c.a);
        }
    "#;

    let program = gl.create_program().unwrap();

    let vert = gl.create_shader(glow::VERTEX_SHADER).unwrap();
    gl.shader_source(vert, vert_src);
    gl.compile_shader(vert);
    if !gl.get_shader_compile_status(vert) {
        panic!("Vertex shader error: {}", gl.get_shader_info_log(vert));
    }

    let frag = gl.create_shader(glow::FRAGMENT_SHADER).unwrap();
    gl.shader_source(frag, frag_src);
    gl.compile_shader(frag);
    if !gl.get_shader_compile_status(frag) {
        panic!("Fragment shader error: {}", gl.get_shader_info_log(frag));
    }

    gl.attach_shader(program, vert);
    gl.attach_shader(program, frag);
    gl.link_program(program);
    if !gl.get_program_link_status(program) {
        panic!("Program link error: {}", gl.get_program_info_log(program));
    }

    gl.delete_shader(vert);
    gl.delete_shader(frag);

    program
}
