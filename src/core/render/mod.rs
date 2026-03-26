use glow::HasContext;
use glam::{Mat4, Vec2};
use hecs::World;
use std::mem;
use wasm_bindgen::JsCast;

use crate::config::GameConfig;
use crate::ecs::components::*;

const BASIC_VERT: &str = r#"
    #version 300 es
    precision highp float;

    layout (location = 0) in vec2 a_position;
    uniform mat4 u_view_proj;
    uniform vec2 u_translation;
    uniform float u_rotation;
    uniform vec2 u_scale;
    uniform float u_aspect;

    void main() {
        float c = cos(u_rotation);
        float s = sin(u_rotation);
        vec2 rotated = vec2(
            a_position.x * c - a_position.y * s,
            a_position.x * s + a_position.y * c
        );
        rotated *= u_scale;
        rotated += u_translation;
        gl_Position = u_view_proj * vec4(rotated, 0.0, 1.0);
    }
"#;

const BASIC_FRAG: &str = r#"
    #version 300 es
    precision highp float;

    uniform vec4 u_color;
    out vec4 out_color;

    void main() {
        out_color = u_color;
    }
"#;

const GAUSSIAN_BLUR_VERT: &str = r#"
    #version 300 es
    precision highp float;

    layout (location = 0) in vec2 a_position;
    out vec2 v_uv;

    void main() {
        v_uv = a_position * 0.5 + 0.5;
        gl_Position = vec4(a_position, 0.0, 1.0);
    }
"#;

const GAUSSIAN_BLUR_FRAG: &str = r#"
    #version 300 es
    precision highp float;

    in vec2 v_uv;
    out vec4 out_color;

    uniform sampler2D u_texture;
    uniform vec2 u_dir;
    uniform int u_radius;
    uniform vec2 u_resolution;

    void main() {
        vec2 texel_size = 1.0 / u_resolution;
        vec4 color = vec4(0.0);
        float total = 0.0;

        for (int i = -u_radius; i <= u_radius; i++) {
            float weight = exp(-float(i * i) / (2.0 * float(u_radius) / 2.0));
            color += texture(u_texture, v_uv + vec2(i) * u_dir * texel_size) * weight;
            total += weight;
        }

        out_color = color / total;
    }
"#;

pub struct Renderer {
    gl: glow::Context,
    canvas: web_sys::HtmlCanvasElement,
    basic_program: glow::Program,
    blur_program: glow::Program,
    quad_vao: glow::VertexArray,
    quad_vbo: glow::Buffer,
    ship_vertices: Vec<Vec2>,
    blur_fbo: glow::Framebuffer,
    blur_texture: glow::Texture,
    screen_width: f32,
    screen_height: f32,
    view_proj: Mat4,
    config: GameConfig,
}

impl Renderer {
    pub fn new(canvas: web_sys::HtmlCanvasElement, config: &GameConfig) -> Result<Self, String> {
        let gl = unsafe {
            glow::Context::from_webgl2_context(
                canvas
                    .get_context("webgl2")
                    .ok_or_else(|| "无法获取 WebGL2 上下文".to_string())?
                    .dyn_into::<web_sys::WebGl2RenderingContext>()
                    .map_err(|_| "WebGL2 上下文类型转换失败".to_string())?,
            )
        };

        let basic_program = unsafe { Self::create_program(&gl, BASIC_VERT, BASIC_FRAG)? };
        let blur_program = unsafe { Self::create_program(&gl, GAUSSIAN_BLUR_VERT, GAUSSIAN_BLUR_FRAG)? };

        let quad_vertices: [f32; 8] = [
            -1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0,
        ];
        let quad_vao = unsafe { gl.create_vertex_array().ok_or("创建 VAO 失败")? };
        let quad_vbo = unsafe { gl.create_buffer().ok_or("创建 VBO 失败")? };
        unsafe {
            gl.bind_vertex_array(Some(quad_vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(quad_vbo));
            gl.buffer_data_size(
                glow::ARRAY_BUFFER,
                mem::size_of::<[f32; 8]>() as i32,
                glow::STATIC_DRAW,
            );
            gl.buffer_sub_data_u8_slice(
                glow::ARRAY_BUFFER,
                0,
                bytemuck::cast_slice(&quad_vertices),
            );
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 0, 0);
            gl.bind_vertex_array(None);
        }

        let blur_fbo = unsafe { gl.create_framebuffer().ok_or("创建 FBO 失败")? };
        let blur_texture = unsafe { gl.create_texture().ok_or("创建纹理失败")? };
        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(blur_texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                1,
                1,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                None,
            );
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(blur_fbo));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(blur_texture),
                0,
            );
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.bind_texture(glow::TEXTURE_2D, None);
        }

        let ship_vertices = vec![
            Vec2::new(0.0, 1.0),
            Vec2::new(-0.6, -0.5),
            Vec2::new(0.6, -0.5),
        ];

        let screen_width = canvas.client_width() as f32;
        let screen_height = canvas.client_height() as f32;

        let view_proj = Mat4::orthographic_rh_gl(
            0.0,
            config.world_width,
            0.0,
            config.world_height,
            -1.0,
            1.0,
        );

        Ok(Self {
            gl,
            canvas,
            basic_program,
            blur_program,
            quad_vao,
            quad_vbo,
            ship_vertices,
            blur_fbo,
            blur_texture,
            screen_width,
            screen_height,
            view_proj,
            config: config.clone(),
        })
    }

    unsafe fn create_program(
        gl: &glow::Context,
        vert_src: &str,
        frag_src: &str,
    ) -> Result<glow::Program, String> {
        let vert = Self::compile_shader(gl, glow::VERTEX_SHADER, vert_src)?;
        let frag = Self::compile_shader(gl, glow::FRAGMENT_SHADER, frag_src)?;

        let program = gl.create_program().ok_or("创建着色器程序失败")?;
        gl.attach_shader(program, vert);
        gl.attach_shader(program, frag);
        gl.link_program(program);

        if !gl.get_program_link_status(program) {
            let log = gl.get_program_info_log(program);
            gl.delete_program(program);
            gl.delete_shader(vert);
            gl.delete_shader(frag);
            return Err(format!("着色器链接失败: {}", log));
        }

        gl.delete_shader(vert);
        gl.delete_shader(frag);
        Ok(program)
    }

    unsafe fn compile_shader(
        gl: &glow::Context,
        shader_type: u32,
        source: &str,
    ) -> Result<glow::Shader, String> {
        let shader = gl.create_shader(shader_type).ok_or("创建着色器失败")?;
        gl.shader_source(shader, source);
        gl.compile_shader(shader);

        if !gl.get_shader_compile_status(shader) {
            let log = gl.get_shader_info_log(shader);
            gl.delete_shader(shader);
            return Err(format!("着色器编译失败: {}", log));
        }

        Ok(shader)
    }

    pub fn resize(&mut self, width: f32, height: f32, dpr: f32) {
        let physical_width = (width * dpr) as i32;
        let physical_height = (height * dpr) as i32;

        if physical_width == self.screen_width as i32 && physical_height == self.screen_height as i32 {
            return;
        }

        self.screen_width = physical_width as f32;
        self.screen_height = physical_height as f32;

        self.canvas.set_width(physical_width as u32);
        self.canvas.set_height(physical_height as u32);

        unsafe {
            self.gl.viewport(0, 0, physical_width, physical_height);

            self.gl.bind_texture(glow::TEXTURE_2D, Some(self.blur_texture));
            self.gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                physical_width,
                physical_height,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                None,
            );
            self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
            self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
            self.gl.bind_texture(glow::TEXTURE_2D, None);
        }

        let world_width = self.config.world_width;
        let world_height = self.config.world_height;
        let screen_aspect = width / height;
        let world_aspect = world_width / world_height;

        let (view_width, view_height) = if screen_aspect > world_aspect {
            (world_height * screen_aspect, world_height)
        } else {
            (world_width, world_width / screen_aspect)
        };

        self.view_proj = Mat4::orthographic_rh_gl(
            (world_width - view_width) / 2.0,
            (world_width + view_width) / 2.0,
            (world_height - view_height) / 2.0,
            (world_height + view_height) / 2.0,
            -1.0,
            1.0,
        );
    }

    pub fn render(&mut self, world: &World, config: &GameConfig) {
        let is_context_lost = self
            .canvas
            .get_context("webgl2")
            .and_then(|ctx| {
                ctx.dyn_into::<web_sys::WebGl2RenderingContext>().ok()
            })
            .map(|gl| gl.is_context_lost())
            .unwrap_or(true);

        if is_context_lost {
            return;
        }

        unsafe {
            self.gl.clear_color(0.02, 0.02, 0.06, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);

            self.gl.use_program(Some(self.basic_program));

            let view_proj_ptr = self.gl.get_uniform_location(self.basic_program, "u_view_proj").as_ref();
            self.gl.uniform_matrix_4_f32_slice(view_proj_ptr, false, &self.view_proj.to_cols_array());

            for layer in [RenderLayer::Bullet, RenderLayer::Ship, RenderLayer::Effect] {
                for (_entity, (transform, renderable)) in
                    world.query::<(&Transform, &Renderable)>().iter()
                {
                    if !renderable.visible || renderable.layer != layer {
                        continue;
                    }

                    let has_bullet = world.get::<&Bullet>(_entity).is_ok();
                    let has_ship = world.get::<&Ship>(_entity).is_ok();
                    let has_effect = world.get::<&Effect>(_entity).is_ok();

                    if has_bullet {
                        self.render_bullet(transform, renderable);
                    } else if has_ship {
                        self.render_ship(transform, renderable);
                    } else if has_effect {
                        self.render_effect(transform, renderable);
                    }
                }
            }

            self.render_boundary(config);

            self.gl.use_program(None);
        }
    }

    unsafe fn render_ship(&mut self, transform: &Transform, renderable: &Renderable) {
        let mut vertices: Vec<f32> = Vec::with_capacity(self.ship_vertices.len() * 2);
        for v in &self.ship_vertices {
            vertices.push(v.x);
            vertices.push(v.y);
        }

        let vbo = self.gl.create_buffer().unwrap();
        self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        self.gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice(&vertices),
            glow::STATIC_DRAW,
        );
        self.gl.enable_vertex_attrib_array(0);
        self.gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 0, 0);

        let trans_ptr = self.gl.get_uniform_location(self.basic_program, "u_translation").as_ref();
        self.gl.uniform_2_f32(trans_ptr, transform.position.x, transform.position.y);

        let rot_ptr = self.gl.get_uniform_location(self.basic_program, "u_rotation").as_ref();
        self.gl.uniform_1_f32(rot_ptr, transform.rotation);

        let scale_ptr = self.gl.get_uniform_location(self.basic_program, "u_scale").as_ref();
        self.gl.uniform_2_f32(
            scale_ptr,
            transform.scale.x * self.config.ship_size,
            transform.scale.y * self.config.ship_size,
        );

        let color_ptr = self.gl.get_uniform_location(self.basic_program, "u_color").as_ref();
        self.gl.uniform_4_f32_slice(color_ptr, &renderable.color);

        self.gl.draw_arrays(glow::TRIANGLES, 0, 3);

        self.gl.delete_buffer(vbo);
        self.gl.bind_buffer(glow::ARRAY_BUFFER, None);
    }

    unsafe fn render_bullet(&mut self, transform: &Transform, renderable: &Renderable) {
        self.render_circle(transform.position, transform.scale.x, renderable.color);
    }

    unsafe fn render_effect(&mut self, transform: &Transform, renderable: &Renderable) {
        self.render_circle(transform.position, transform.scale.x, renderable.color);
    }

    unsafe fn render_circle(&mut self, center: Vec2, radius: f32, color: [f32; 4]) {
        let segments = 16;
        let mut vertices: Vec<f32> = Vec::with_capacity((segments + 2) * 2);

        vertices.push(center.x);
        vertices.push(center.y);

        for i in 0..=segments {
            let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
            vertices.push(center.x + radius * angle.cos());
            vertices.push(center.y + radius * angle.sin());
        }

        let vbo = self.gl.create_buffer().unwrap();
        self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        self.gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice(&vertices),
            glow::STATIC_DRAW,
        );
        self.gl.enable_vertex_attrib_array(0);
        self.gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 0, 0);

        let trans_ptr = self.gl.get_uniform_location(self.basic_program, "u_translation").as_ref();
        self.gl.uniform_2_f32(trans_ptr, 0.0, 0.0);

        let rot_ptr = self.gl.get_uniform_location(self.basic_program, "u_rotation").as_ref();
        self.gl.uniform_1_f32(rot_ptr, 0.0);

        let scale_ptr = self.gl.get_uniform_location(self.basic_program, "u_scale").as_ref();
        self.gl.uniform_2_f32(scale_ptr, 1.0, 1.0);

        let color_ptr = self.gl.get_uniform_location(self.basic_program, "u_color").as_ref();
        self.gl.uniform_4_f32_slice(color_ptr, &color);

        self.gl.draw_arrays(glow::TRIANGLE_FAN, 0, segments + 2);

        self.gl.delete_buffer(vbo);
        self.gl.bind_buffer(glow::ARRAY_BUFFER, None);
    }

    unsafe fn render_boundary(&mut self, config: &GameConfig) {
        let w = config.world_width;
        let h = config.world_height;

        let vertices: [f32; 8] = [
            0.0, 0.0,  w, 0.0,  w, h,  0.0, h,
        ];

        let vbo = self.gl.create_buffer().unwrap();
        self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        self.gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice(&vertices),
            glow::STATIC_DRAW,
        );
        self.gl.enable_vertex_attrib_array(0);
        self.gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 0, 0);

        let trans_ptr = self.gl.get_uniform_location(self.basic_program, "u_translation").as_ref();
        self.gl.uniform_2_f32(trans_ptr, 0.0, 0.0);

        let rot_ptr = self.gl.get_uniform_location(self.basic_program, "u_rotation").as_ref();
        self.gl.uniform_1_f32(rot_ptr, 0.0);

        let scale_ptr = self.gl.get_uniform_location(self.basic_program, "u_scale").as_ref();
        self.gl.uniform_2_f32(scale_ptr, 1.0, 1.0);

        let color_ptr = self.gl.get_uniform_location(self.basic_program, "u_color").as_ref();
        self.gl.uniform_4_f32_slice(color_ptr, &[0.3, 0.3, 0.5, 1.0]);

        self.gl.draw_arrays(glow::LINE_LOOP, 0, 4);

        self.gl.delete_buffer(vbo);
        self.gl.bind_buffer(glow::ARRAY_BUFFER, None);
        self.gl.use_program(None);
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_program(self.basic_program);
            self.gl.delete_program(self.blur_program);
            self.gl.delete_vertex_array(self.quad_vao);
            self.gl.delete_buffer(self.quad_vbo);
            self.gl.delete_framebuffer(self.blur_fbo);
            self.gl.delete_texture(self.blur_texture);
        }
    }
}
