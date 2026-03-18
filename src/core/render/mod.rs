use glow::HasContext;
use glam::{Mat4, Vec2};
use hecs::World;
use std::mem;
use wasm_bindgen::JsCast;

use crate::config::GameConfig;
use crate::ecs::components::*;

// 基础几何渲染顶点着色器
const BASIC_VERT: &str = r#"
    #version 300 es
    precision highp float;

    layout (location = 0) in vec2 a_position;
    uniform mat4 u_view_proj;
    uniform vec2 u_offset;
    uniform float u_scale;
    uniform vec4 u_color;

    out vec4 v_color;

    void main() {
        vec2 pos = a_position * u_scale + u_offset;
        gl_Position = u_view_proj * vec4(pos, 0.0, 1.0);
        v_color = u_color;
    }
"#;

// 基础纯色渲染片段着色器
const BASIC_FRAG: &str = r#"
    #version 300 es
    precision highp float;

    in vec4 v_color;
    out vec4 out_color;

    void main() {
        out_color = v_color;
    }
"#;

// 高斯模糊顶点着色器（背景特效用）
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

// 高斯模糊片段着色器
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

// 渲染器核心结构体
pub struct Renderer {
    gl: glow::Context,
    canvas: web_sys::HtmlCanvasElement,
    // 着色器程序
    basic_program: glow::Program,
    blur_program: glow::Program,
    // 顶点缓冲区
    quad_vao: glow::VertexArray,
    quad_vbo: glow::Buffer,
    // 飞船等腰三角形顶点数据
    ship_vertices: Vec<Vec2>,
    // 高斯模糊帧缓冲区
    blur_fbo: glow::Framebuffer,
    blur_texture: glow::Texture,
    // 渲染状态
    screen_width: i32,
    screen_height: i32,
    config: GameConfig,
    // 相机视图投影矩阵
    view_proj: Mat4,
    // 星云数据
    nebula_positions: Vec<Vec2>,
    // WebGL上下文丢失标记
    context_lost: bool,
}

impl Renderer {
    // 初始化渲染器，创建WebGL上下文、编译着色器
    pub fn new(canvas: web_sys::HtmlCanvasElement, config: &GameConfig) -> Result<Self, String> {
        // 获取WebGL2上下文，兼容安卓99%以上设备
        let gl = canvas
            .get_context("webgl2")
            .map_err(|_| "无法获取WebGL2上下文".to_string())?
            .ok_or("当前设备不支持WebGL2".to_string())?
            .dyn_into::<web_sys::WebGl2RenderingContext>()
            .map_err(|_| "WebGL2上下文类型转换失败".to_string())?;

        let gl = glow::Context::from_webgl2(gl);

        // 编译着色器程序
        let basic_program = compile_program(&gl, BASIC_VERT, BASIC_FRAG)?;
        let blur_program = compile_program(&gl, GAUSSIAN_BLUR_VERT, GAUSSIAN_BLUR_FRAG)?;

        // 创建全屏四边形VAO（用于模糊渲染）
        let quad_vertices: [f32; 12] = [
            -1.0, -1.0, 1.0, -1.0, 1.0, 1.0,
            -1.0, -1.0, 1.0, 1.0, -1.0, 1.0,
        ];

        unsafe {
            let quad_vao = gl.create_vertex_array()?;
            let quad_vbo = gl.create_buffer()?;

            gl.bind_vertex_array(Some(quad_vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(quad_vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                &quad_vertices.align_to::<u8>().1,
                glow::STATIC_DRAW,
            );

            gl.vertex_attrib_pointer_f32(
                0,
                2,
                glow::FLOAT,
                false,
                2 * mem::size_of::<f32>() as i32,
                0,
            );
            gl.enable_vertex_attrib_array(0);

            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);

            // 创建模糊帧缓冲区和纹理
            let blur_fbo = gl.create_framebuffer()?;
            let blur_texture = gl.create_texture()?;

            // 生成等腰三角形飞船顶点（纸飞机样式，机头朝前）
            let ship_size = config.ship_size;
            let ship_vertices = vec![
                Vec2::new(ship_size, 0.0),         // 机头
                Vec2::new(-ship_size / 2.0, ship_size / 2.0), // 机尾左上
                Vec2::new(-ship_size / 2.0, -ship_size / 2.0), // 机尾左下
            ];

            // 启用混合，处理透明度
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            // 禁用深度测试，2D渲染不需要
            gl.disable(glow::DEPTH_TEST);

            // 初始化相机矩阵
            let view_proj = Mat4::orthographic_rh_gl(
                0.0,
                config.world_width,
                0.0,
                config.world_height,
                -1.0,
                1.0,
            );

            // 生成星云位置
            let mut nebula_positions = Vec::new();
            for i in 0..config.nebula_count {
                let x = ((i * 17 + 31) % 100) as f32 / 100.0 * config.world_width;
                let y = ((i * 23 + 47) % 100) as f32 / 100.0 * config.world_height;
                nebula_positions.push(Vec2::new(x, y));
            }

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
                screen_width: 0,
                screen_height: 0,
                config: config.clone(),
                view_proj,
                nebula_positions,
                context_lost: false,
            })
        }
    }

    // 屏幕尺寸变化时更新，适配安卓旋转屏幕
    pub fn resize(&mut self, width: f32, height: f32, dpr: f32) {
        let physical_width = (width * dpr) as i32;
        let physical_height = (height * dpr) as i32;

        if self.screen_width == physical_width && self.screen_height == physical_height {
            return;
        }

        self.screen_width = physical_width;
        self.screen_height = physical_height;

        // 更新画布尺寸
        self.canvas.set_width(physical_width as u32);
        self.canvas.set_height(physical_height as u32);

        // 更新视口
        unsafe {
            self.gl.viewport(0, 0, physical_width, physical_height);

            // 更新模糊纹理尺寸
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
            self.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as i32,
            );
            self.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );
            self.gl.bind_texture(glow::TEXTURE_2D, None);
        }

        // 更新相机投影矩阵，适配屏幕宽高比
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

    // 主渲染入口，按层级渲染所有游戏内容
    pub fn render(&mut self, world: &World, config: &GameConfig) {
        // 检查WebGL上下文是否丢失
        if self.context_lost {
            return;
        }

        unsafe {
            // 清空画布为深蓝色背景
            self.gl.clear_color(0.02, 0.02, 0.08, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);

            // 渲染背景星云
            self.render_nebula();

            // 渲染战场边界
            self.render_boundary(config);

            // 收集所有可渲染实体，按层级排序
            let mut renderables = Vec::new();
            for (entity, (transform, renderable)) in world.query::<(&Transform, &Renderable)>().iter() {
                if !renderable.visible {
                    continue;
                }
                renderables.push((entity, transform, renderable));
            }

            // 按渲染层级从后往前渲染，保证层级正确
            renderables.sort_by_key(|(_, _, r)| r.layer);

            // 遍历渲染所有实体
            for (entity, transform, renderable) in renderables {
                // 渲染飞船
                if world.get::<FactionComponent>(entity).is_ok() {
                    self.render_ship(transform, renderable);
                }
                // 渲染子弹
                else if world.get::<Bullet>(entity).is_ok() {
                    self.render_bullet(transform, renderable);
                }
                // 渲染爆炸特效
                else if let Ok(effect) = world.get::<Effect>(entity) {
                    let progress = effect.lifetime.as_secs_f32() / effect.max_lifetime.as_secs_f32();
                    let current_scale = effect.start_scale + (effect.end_scale - effect.start_scale) * progress;
                    let mut color = renderable.color;
                    color[3] = 1.0 - progress; // 淡出效果
                    self.render_circle(transform.position, current_scale, color);
                }
            }
        }
    }

    // 渲染背景星云
    unsafe fn render_nebula(&mut self) {
        for pos in &self.nebula_positions {
            let color = [0.1, 0.1, 0.2, 0.3];
            self.render_circle(*pos, 3.0, color);
        }
    }

    // 渲染战场边界
    unsafe fn render_boundary(&mut self, config: &GameConfig) {
        let boundary_color = [0.3, 0.3, 0.4, 0.5];
        let line_width = 0.1;

        // 底部边界
        self.render_rect(
            Vec2::new(config.world_width / 2.0, line_width / 2.0),
            Vec2::new(config.world_width, line_width),
            boundary_color,
        );
        // 顶部边界
        self.render_rect(
            Vec2::new(config.world_width / 2.0, config.world_height - line_width / 2.0),
            Vec2::new(config.world_width, line_width),
            boundary_color,
        );
        // 左侧边界
        self.render_rect(
            Vec2::new(line_width / 2.0, config.world_height / 2.0),
            Vec2::new(line_width, config.world_height),
            boundary_color,
        );
        // 右侧边界
        self.render_rect(
            Vec2::new(config.world_width - line_width / 2.0, config.world_height / 2.0),
            Vec2::new(line_width, config.world_height),
            boundary_color,
        );
    }

    // 渲染矩形
    unsafe fn render_rect(&mut self, position: Vec2, size: Vec2, color: [f32; 4]) {
        self.gl.use_program(Some(self.basic_program));

        let vertices = [
            Vec2::new(-0.5, -0.5),
            Vec2::new(0.5, -0.5),
            Vec2::new(0.5, 0.5),
            Vec2::new(-0.5, -0.5),
            Vec2::new(0.5, 0.5),
            Vec2::new(-0.5, 0.5),
        ];

        let vbo = self.gl.create_buffer().unwrap();
        self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        self.gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            &vertices.align_to::<u8>().1,
            glow::STREAM_DRAW,
        );

        self.gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 2 * mem::size_of::<f32>() as i32, 0);
        self.gl.enable_vertex_attrib_array(0);

        // 应用缩放
        let scale_matrix = Mat4::from_scale(size.extend(1.0));
        let mvp = self.view_proj * Mat4::from_translation(position.extend(0.0)) * scale_matrix;
        self.gl.uniform_matrix_4_f32_slice(
            self.gl.get_uniform_location(self.basic_program, "u_view_proj").as_ref(),
            false,
            &mvp.to_cols_array(),
        );
        self.gl.uniform_2_f32(
            self.gl.get_uniform_location(self.basic_program, "u_offset").as_ref(),
            0.0,
            0.0,
        );
        self.gl.uniform_1_f32(
            self.gl.get_uniform_location(self.basic_program, "u_scale").as_ref(),
            1.0,
        );
        self.gl.uniform_4_f32_slice(
            self.gl.get_uniform_location(self.basic_program, "u_color").as_ref(),
            &color,
        );

        self.gl.draw_arrays(glow::TRIANGLES, 0, 6);

        self.gl.delete_buffer(vbo);
        self.gl.bind_buffer(glow::ARRAY_BUFFER, None);
        self.gl.use_program(None);
    }

    // 渲染飞船（等腰三角形纸飞机）
    unsafe fn render_ship(&mut self, transform: &Transform, renderable: &Renderable) {
        self.gl.use_program(Some(self.basic_program));

        // 顶点缓冲区
        let vbo = self.gl.create_buffer().unwrap();
        self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        self.gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            &self.ship_vertices.align_to::<u8>().1,
            glow::STREAM_DRAW,
        );

        self.gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 2 * mem::size_of::<f32>() as i32, 0);
        self.gl.enable_vertex_attrib_array(0);

        // 计算旋转后的MVP矩阵
        let rotation = Mat4::from_rotation_z(transform.rotation);
        let model = Mat4::from_translation(transform.position.extend(0.0)) * rotation;
        let mvp = self.view_proj * model;

        // 设置着色器uniform
        self.gl.uniform_matrix_4_f32_slice(
            self.gl.get_uniform_location(self.basic_program, "u_view_proj").as_ref(),
            false,
            &mvp.to_cols_array(),
        );
        self.gl.uniform_2_f32(
            self.gl.get_uniform_location(self.basic_program, "u_offset").as_ref(),
            0.0,
            0.0,
        );
        self.gl.uniform_1_f32(
            self.gl.get_uniform_location(self.basic_program, "u_scale").as_ref(),
            1.0,
        );
        self.gl.uniform_4_f32_slice(
            self.gl.get_uniform_location(self.basic_program, "u_color").as_ref(),
            &renderable.color,
        );

        // 绘制三角形
        self.gl.draw_arrays(glow::TRIANGLES, 0, 3);

        // 清理临时资源
        self.gl.delete_buffer(vbo);
        self.gl.bind_buffer(glow::ARRAY_BUFFER, None);
        self.gl.use_program(None);
    }

    // 渲染子弹（圆形）
    unsafe fn render_bullet(&mut self, transform: &Transform, renderable: &Renderable) {
        self.render_circle(transform.position, transform.scale.x, renderable.color);
    }

    // 通用圆形渲染工具函数
    unsafe fn render_circle(&mut self, position: Vec2, radius: f32, color: [f32; 4]) {
        self.gl.use_program(Some(self.basic_program));

        // 生成圆形顶点（16个分段，足够平滑且性能好）
        let segments = 16;
        let mut vertices = Vec::with_capacity(segments + 1);
        vertices.push(Vec2::ZERO); // 圆心
        for i in 0..=segments {
            let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
            vertices.push(Vec2::new(angle.cos(), angle.sin()));
        }

        // 顶点缓冲区
        let vbo = self.gl.create_buffer().unwrap();
        self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        self.gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            &vertices.align_to::<u8>().1,
            glow::STREAM_DRAW,
        );

        self.gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 2 * mem::size_of::<f32>() as i32, 0);
        self.gl.enable_vertex_attrib_array(0);

        // 设置着色器uniform
        self.gl.uniform_matrix_4_f32_slice(
            self.gl.get_uniform_location(self.basic_program, "u_view_proj").as_ref(),
            false,
            &self.view_proj.to_cols_array(),
        );
        self.gl.uniform_2_f32(
            self.gl.get_uniform_location(self.basic_program, "u_offset").as_ref(),
            position.x,
            position.y,
        );
        self.gl.uniform_1_f32(
            self.gl.get_uniform_location(self.basic_program, "u_scale").as_ref(),
            radius,
        );
        self.gl.uniform_4_f32_slice(
            self.gl.get_uniform_location(self.basic_program, "u_color").as_ref(),
            &color,
        );

        // 绘制扇形
        self.gl.draw_arrays(glow::TRIANGLE_FAN, 0, vertices.len() as i32);

        // 清理资源
        self.gl.delete_buffer(vbo);
        self.gl.bind_buffer(glow::ARRAY_BUFFER, None);
        self.gl.use_program(None);
    }
}

// 着色器编译辅助函数
fn compile_shader(gl: &glow::Context, shader_type: u32, source: &str) -> Result<glow::Shader, String> {
    unsafe {
        let shader = gl.create_shader(shader_type)?;
        gl.shader_source(shader, source);
        gl.compile_shader(shader);

        if !gl.get_shader_compile_status(shader) {
            let log = gl.get_shader_info_log(shader);
            gl.delete_shader(shader);
            return Err(format!("着色器编译失败: {}", log));
        }

        Ok(shader)
    }
}

// 着色器程序链接辅助函数
fn compile_program(gl: &glow::Context, vert_source: &str, frag_source: &str) -> Result<glow::Program, String> {
    unsafe {
        let vert_shader = compile_shader(gl, glow::VERTEX_SHADER, vert_source)?;
        let frag_shader = compile_shader(gl, glow::FRAGMENT_SHADER, frag_source)?;

        let program = gl.create_program()?;
        gl.attach_shader(program, vert_shader);
        gl.attach_shader(program, frag_shader);
        gl.link_program(program);

        if !gl.get_program_link_status(program) {
            let log = gl.get_program_info_log(program);
            gl.delete_program(program);
            gl.delete_shader(vert_shader);
            gl.delete_shader(frag_shader);
            return Err(format!("程序链接失败: {}", log));
        }

        gl.delete_shader(vert_shader);
        gl.delete_shader(frag_shader);

        Ok(program)
    }
}

// 资源自动释放，避免WebGL内存泄漏
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
