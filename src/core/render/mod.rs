use bytemuck::{Pod, Zeroable};
use glow::*;
use glam::{Mat4, Vec2};
use hecs::World;
use std::mem;
use wasm_bindgen::JsCast;
use crate::config::GameConfig;
use crate::ecs::components::*;

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}

pub struct Renderer {
    gl: Context,
    program: NativeProgram,
    vao: NativeVertexArray,
    vbo: NativeBuffer,
    u_projection: NativeUniformLocation,
    u_translation: NativeUniformLocation,
    u_rotation: NativeUniformLocation,
    u_scale: NativeUniformLocation,
    canvas_width: f32,
    canvas_height: f32,
}

impl Renderer {
    pub fn new(canvas: &web_sys::HtmlCanvasElement, _config: &GameConfig) -> Result<Self, String> {
        // 正确处理canvas上下文获取
        let context = canvas
            .get_context("webgl2")
            .map_err(|e| format!("获取 WebGL2 上下文失败: {:?}", e))?
            .ok_or("无法获取 WebGL2 上下文".to_string())?;
        
        // 类型转换
        let gl_context = context
            .dyn_into::<web_sys::WebGl2RenderingContext>()
            .map_err(|_| "转换为 WebGl2RenderingContext 失败".to_string())?;
        
        let gl = unsafe { Context::from_webgl2_context(gl_context) };

        // 着色器源码
        let vertex_shader_source = r#"#version 300 es
            precision mediump float;
            layout(location = 0) in vec2 a_position;
            layout(location = 1) in vec4 a_color;
            uniform mat4 u_projection;
            uniform vec2 u_translation;
            uniform float u_rotation;
            uniform vec2 u_scale;
            out vec4 v_color;
            void main() {
                float c = cos(u_rotation);
                float s = sin(u_rotation);
                mat2 rot = mat2(c, s, -s, c);
                vec2 pos = rot * (a_position * u_scale) + u_translation;
                gl_Position = u_projection * vec4(pos, 0.0, 1.0);
                v_color = a_color;
            }
        "#;
        let fragment_shader_source = r#"#version 300 es
            precision mediump float;
            in vec4 v_color;
            out vec4 frag_color;
            void main() {
                frag_color = v_color;
            }
        "#;

        // 编译着色器
        let vertex_shader = unsafe { Self::compile_shader(&gl, VERTEX_SHADER, vertex_shader_source)? };
        let fragment_shader = unsafe { Self::compile_shader(&gl, FRAGMENT_SHADER, fragment_shader_source)? };

        // 链接着色器程序
        let program = unsafe {
            let program = gl
                .create_program()
                .ok_or("创建着色器程序失败".to_string())?;
            gl.attach_shader(program, vertex_shader);
            gl.attach_shader(program, fragment_shader);
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                let log = gl.get_program_info_log(program);
                gl.delete_program(program);
                return Err(format!("着色器程序链接失败: {}", log));
            }
            gl.delete_shader(vertex_shader);
            gl.delete_shader(fragment_shader);
            program
        };

        unsafe {
            gl.use_program(Some(program));
        }

        // 创建VAO/VBO
        let vao = unsafe {
            gl.create_vertex_array()
                .ok_or("创建 VAO 失败".to_string())?
        };
        let vbo = unsafe {
            gl.create_buffer()
                .ok_or("创建 VBO 失败".to_string())?
        };

        // 配置VAO
        unsafe {
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(ARRAY_BUFFER, Some(vbo));
            let stride = mem::size_of::<Vertex>() as i32;
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, FLOAT, false, stride, 0);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 4, FLOAT, false, stride, 2 * 4);
            gl.bind_vertex_array(None);
        }

        // 获取Uniform位置
        let u_projection = unsafe {
            gl.get_uniform_location(program, "u_projection")
                .ok_or("无法获取 u_projection uniform 位置".to_string())?
        };
        let u_translation = unsafe {
            gl.get_uniform_location(program, "u_translation")
                .ok_or("无法获取 u_translation uniform 位置".to_string())?
        };
        let u_rotation = unsafe {
            gl.get_uniform_location(program, "u_rotation")
                .ok_or("无法获取 u_rotation uniform 位置".to_string())?
        };
        let u_scale = unsafe {
            gl.get_uniform_location(program, "u_scale")
                .ok_or("无法获取 u_scale uniform 位置".to_string())?
        };

        Ok(Self {
            gl,
            program,
            vao,
            vbo,
            u_projection,
            u_translation,
            u_rotation,
            u_scale,
            canvas_width: 100.0,
            canvas_height: 100.0,
        })
    }

    unsafe fn compile_shader(
        gl: &Context,
        shader_type: NativeShaderType,
        source: &str,
    ) -> Result<NativeShader, String> {
        let shader = gl
            .create_shader(shader_type)
            .ok_or("创建着色器失败".to_string())?;
        gl.shader_source(shader, source);
        gl.compile_shader(shader);
        if !gl.get_shader_compile_status(shader) {
            let log = gl.get_shader_info_log(shader);
            gl.delete_shader(shader);
            return Err(format!("着色器编译失败: {}", log));
        }
        Ok(shader)
    }

    pub fn resize(&mut self, width: f32, height: f32, _dpr: f32) {
        self.canvas_width = width;
        self.canvas_height = height;
    }

    pub fn render(&self, world: &World, config: &GameConfig) {
        unsafe {
            let gl = &self.gl;
            gl.viewport(0, 0, self.canvas_width as i32, self.canvas_height as i32);
            gl.clear_color(0.02, 0.02, 0.05, 1.0);
            gl.clear(COLOR_BUFFER_BIT);
            gl.use_program(Some(self.program));

            // 设置投影矩阵
            let projection = Mat4::orthographic_rh(
                0.0,
                config.world_width,
                0.0,
                config.world_height,
                -1.0,
                1.0,
            );
            gl.uniform_matrix_4_f32_slice(Some(&self.u_projection), false, &projection.to_cols_array());

            // 渲染边界
            self.render_boundary(config);

            // 渲染所有实体
            for (_entity, (transform, renderable)) in
                world.query::<(&Transform, &Renderable)>().iter()
            {
                if !renderable.visible {
                    continue;
                }
                match renderable.layer {
                    RenderLayer::Ship => {
                        self.render_ship(transform, &renderable.color);
                    }
                    RenderLayer::Bullet => {
                        self.render_circle(transform, &renderable.color, config.bullet_size);
                    }
                    RenderLayer::Effect => {
                        if let Some(effect) = world.get::<&Effect>(_entity).ok() {
                            let progress = 1.0
                                - (effect.lifetime.as_secs_f32()
                                    / effect.max_lifetime.as_secs_f32());
                            let scale = effect.start_scale
                                + (effect.end_scale - effect.start_scale) * progress;
                            self.render_circle(
                                transform,
                                &renderable.color,
                                config.ship_size * scale,
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn render_ship(&self, transform: &Transform, color: &[f32; 4]) {
        unsafe {
            let gl = &self.gl;
            let size = 3.0;
            // 三角形飞船顶点
            let vertices: [Vertex; 3] = [
                Vertex {
                    position: [size, 0.0],
                    color: *color,
                },
                Vertex {
                    position: [size * 0.6 * (-0.5), size * 0.6 * 0.866],
                    color: *color,
                },
                Vertex {
                    position: [size * 0.6 * (-0.5), size * 0.6 * (-0.866)],
                    color: *color,
                },
            ];

            gl.bind_vertex_array(Some(self.vao));
            gl.bind_buffer(ARRAY_BUFFER, Some(self.vbo));
            gl.buffer_data_size(
                ARRAY_BUFFER,
                (vertices.len() * mem::size_of::<Vertex>()) as isize,
                STATIC_DRAW,
            );
            gl.buffer_sub_data_u8_slice(
                ARRAY_BUFFER,
                0,
                bytemuck::cast_slice(&vertices),
            );

            gl.uniform_2_f32(Some(&self.u_translation), transform.position.x, transform.position.y);
            gl.uniform_1_f32(Some(&self.u_rotation), transform.rotation);
            gl.uniform_2_f32(Some(&self.u_scale), transform.scale.x, transform.scale.y);
            gl.draw_arrays(TRIANGLES, 0, 3);
        }
    }

    fn render_circle(&self, transform: &Transform, color: &[f32; 4], radius: f32) {
        unsafe {
            let gl = &self.gl;
            const SEGMENTS: usize = 16;
            let mut vertices = Vec::with_capacity(SEGMENTS + 2);
            
            vertices.push(Vertex {
                position: [0.0, 0.0],
                color: *color,
            });

            for i in 0..=SEGMENTS {
                let angle = (i as f32 / SEGMENTS as f32) * std::f32::consts::TAU;
                vertices.push(Vertex {
                    position: [radius * angle.cos(), radius * angle.sin()],
                    color: *color,
                });
            }

            gl.bind_vertex_array(Some(self.vao));
            gl.bind_buffer(ARRAY_BUFFER, Some(self.vbo));
            gl.buffer_data_size(
                ARRAY_BUFFER,
                (vertices.len() * mem::size_of::<Vertex>()) as isize,
                STATIC_DRAW,
            );
            gl.buffer_sub_data_u8_slice(
                ARRAY_BUFFER,
                0,
                bytemuck::cast_slice(&vertices),
            );

            gl.uniform_2_f32(Some(&self.u_translation), transform.position.x, transform.position.y);
            gl.uniform_1_f32(Some(&self.u_rotation), transform.rotation);
            gl.uniform_2_f32(Some(&self.u_scale), transform.scale.x, transform.scale.y);
            // 修复类型错误：usize转i32
            gl.draw_arrays(TRIANGLE_FAN, 0, (SEGMENTS + 2) as i32);
        }
    }

    fn render_boundary(&self, config: &GameConfig) {
        unsafe {
            let gl = &self.gl;
            let w = config.world_width;
            let h = config.world_height;
            let color = [0.3, 0.3, 0.4, 1.0];
            let vertices: [Vertex; 4] = [
                Vertex { position: [0.0, 0.0], color },
                Vertex { position: [w, 0.0], color },
                Vertex { position: [w, h], color },
                Vertex { position: [0.0, h], color },
            ];

            gl.bind_vertex_array(Some(self.vao));
            gl.bind_buffer(ARRAY_BUFFER, Some(self.vbo));
            gl.buffer_data_size(
                ARRAY_BUFFER,
                (vertices.len() * mem::size_of::<Vertex>()) as isize,
                STATIC_DRAW,
            );
            gl.buffer_sub_data_u8_slice(
                ARRAY_BUFFER,
                0,
                bytemuck::cast_slice(&vertices),
            );

            gl.uniform_2_f32(Some(&self.u_translation), 0.0, 0.0);
            gl.uniform_1_f32(Some(&self.u_rotation), 0.0);
            gl.uniform_2_f32(Some(&self.u_scale), 1.0, 1.0);
            gl.draw_arrays(LINE_LOOP, 0, 4);
        }
    }
}
