use std::{slice, sync::Arc};

use glam::{Mat4, Vec2, Vec3};
use glow::*;
use skia_safe::{Bitmap, ColorType, ImageInfo};

use crate::{
    base_render::{BaseSkinRender, ErrorType, ModelPartType},
    cube::cube,
    cube_model::{CubeModelItemObj},
    model::model,
    renders::gl_render::{
        gl_model::{ModelVao, VaoItem, VertexOpenGL},
        gl_shader::gl_shader,
    },
    texture::texture::{self, SkinType},
};

fn init_shader(gl: &Context) -> Program {
    let mut vertex = String::from(gl_shader::VERTEX_SHADER_SOURCE);
    if cfg!(target_os = "macos") {
        vertex.insert_str(0, gl_shader::MACOS_HEADER);
    }

    let mut fragment = String::from(gl_shader::FRAGMENT_SHADER_SOURCE);
    if cfg!(target_os = "macos") {
        fragment.insert_str(0, gl_shader::MACOS_HEADER);
    }

    unsafe {
        let vertex_shader = gl.create_shader(VERTEX_SHADER).unwrap();

        gl.shader_source(vertex_shader, &vertex);
        gl.compile_shader(vertex_shader);
        if !gl.get_shader_compile_status(vertex_shader) {
            panic!(
                "vertex Shader compile fail: {info}",
                info = gl.get_shader_info_log(vertex_shader)
            );
        }

        let fragment_shader = gl.create_shader(FRAGMENT_SHADER).unwrap();

        gl.shader_source(fragment_shader, &fragment);
        gl.compile_shader(fragment_shader);
        if !gl.get_shader_compile_status(fragment_shader) {
            panic!(
                "fragment Shader compile fail: {info}",
                info = gl.get_shader_info_log(fragment_shader)
            );
        }

        let pg = gl.create_program().unwrap();

        gl.attach_shader(pg, vertex_shader);
        gl.attach_shader(pg, fragment_shader);
        gl.link_program(pg);
        if !gl.get_program_link_status(pg) {
            panic!(
                "Program link fail: {info}",
                info = gl.get_program_info_log(pg)
            );
        }

        gl.detach_shader(pg, vertex_shader);
        gl.detach_shader(pg, fragment_shader);

        gl.delete_shader(vertex_shader);
        gl.delete_shader(fragment_shader);

        pg
    }
}

fn check_error(gl: &Context) {
    unsafe {
        let mut err = gl.get_error();
        while err != glow::NO_ERROR {
            eprintln!("OpenGL Error: {}", err);
            err = gl.get_error();
        }
    }
}

fn change_color_type(image: &Bitmap) -> Option<Bitmap> {
    let rgba_info = ImageInfo::new(
        image.dimensions(),
        ColorType::RGBA8888,
        image.alpha_type(),
        None,
    );
    let mut rgba_bitmap = Bitmap::new();
    rgba_bitmap.set_info(&rgba_info, image.row_bytes());
    rgba_bitmap.alloc_pixels();

    unsafe {
        if image.read_pixels(
            &rgba_info,
            rgba_bitmap.pixels(),
            rgba_bitmap.row_bytes(),
            0,
            0,
        ) {
            Some(rgba_bitmap)
        } else {
            None
        }
    }
}

fn load_tex(is_gles: bool, gl: &glow::Context, image: &mut Bitmap, texture: Texture) {
    unsafe {
        gl.active_texture(TEXTURE0);
        gl.bind_texture(TEXTURE_2D, Some(texture));

        // 设置纹理参数
        gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, NEAREST as i32);
        gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, NEAREST as i32);
        gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_S, CLAMP_TO_BORDER as i32);
        gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_T, CLAMP_TO_BORDER as i32);
    }

    let width = image.width();
    let height = image.height();
    let pixel_data_size = width * height * 4;

    if is_gles && image.color_type() == ColorType::BGRA8888 {
        unsafe {
            let mut image = change_color_type(&image).unwrap();
            let pixels_slice =
                slice::from_raw_parts(image.pixels() as *const u8, pixel_data_size as usize);

            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                width,
                height,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                PixelUnpackData::Slice(Some(pixels_slice)),
            );
        }
    } else {
        let (internal_format, format) = match image.color_type() {
            ColorType::RGBA8888 => (glow::RGBA8, glow::RGBA),
            ColorType::BGRA8888 => (glow::RGBA8, glow::BGRA),
            _ => {
                panic!("Color type error");
            }
        };

        unsafe {
            let pixels_slice =
                slice::from_raw_parts(image.pixels() as *const u8, pixel_data_size as usize);

            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                internal_format as i32,
                width,
                height,
                0,
                format,
                glow::UNSIGNED_BYTE,
                PixelUnpackData::Slice(Some(pixels_slice)),
            );
        }
    }

    unsafe {
        gl.bind_texture(glow::TEXTURE_2D, None);
    }
}

fn put_vao_item(gl: &Context, vao: &VaoItem, model: &CubeModelItemObj, uv: &Vec<f32>, pg: Program) {
    unsafe {
        gl.use_program(Some(pg));

        gl.bind_vertex_array(Some(vao.vertex_array_object));

        let postion = gl.get_attrib_location(pg, "a_position").unwrap();
        let tex = gl.get_attrib_location(pg, "a_texCoord").unwrap();
        let normal = gl.get_attrib_location(pg, "a_normal").unwrap();

        gl.disable_vertex_attrib_array(postion);
        gl.disable_vertex_attrib_array(tex);
        gl.disable_vertex_attrib_array(normal);

        let size = model.model.len() / 3;

        let mut points: Vec<VertexOpenGL> = Vec::new();

        for index in 0..size {
            let src = index * 3;
            let src1 = index * 2;

            points[index] = VertexOpenGL {
                pos: Vec3::new(model.model[src], model.model[src + 1], model.model[src + 2]),
                uv: Vec2::new(uv[src1], uv[src1 + 1]),
                normal: Vec3::new(
                    cube::VERTICES[src],
                    cube::VERTICES[src + 1],
                    cube::VERTICES[src + 2],
                ),
            }
        }

        gl.bind_buffer(ARRAY_BUFFER, Some(vao.vertex_buffer_object));
        let data: &[u8] = core::slice::from_raw_parts(
            points.as_ptr() as *const u8,
            points.len() * core::mem::size_of::<VertexOpenGL>(),
        );
        gl.buffer_data_u8_slice(ARRAY_BUFFER, data, STATIC_DRAW);

        gl.bind_buffer(ARRAY_BUFFER, Some(vao.index_buffer_object));
        let data: &[u8] = core::slice::from_raw_parts(
            model.point.as_ptr() as *const u8,
            model.point.len() * core::mem::size_of::<VertexOpenGL>(),
        );
        gl.buffer_data_u8_slice(ARRAY_BUFFER, data, STATIC_DRAW);

        gl.vertex_attrib_pointer_f32(
            postion,
            3,
            FLOAT,
            false,
            (core::mem::size_of::<f32>() * 8) as i32,
            0,
        );
        gl.vertex_attrib_pointer_f32(
            tex,
            3,
            FLOAT,
            false,
            (core::mem::size_of::<f32>() * 8) as i32,
            (core::mem::size_of::<f32>() * 3) as i32,
        );
        gl.vertex_attrib_pointer_f32(
            normal,
            3,
            FLOAT,
            false,
            (core::mem::size_of::<f32>() * 8) as i32,
            (core::mem::size_of::<f32>() * 5) as i32,
        );

        gl.enable_vertex_attrib_array(postion);
        gl.enable_vertex_attrib_array(tex);
        gl.enable_vertex_attrib_array(normal);

        gl.bind_vertex_array(None);
    }
}

/// OpenGL 皮肤渲染器
pub struct SkinRenderOpenGL {
    pub base: BaseSkinRender,

    gl: Arc<Context>,
    is_gles: bool,

    width: i32,
    height: i32,

    // Shader programs
    pg: Program,

    // Textures
    texture_skin: Texture,
    texture_cape: Texture,

    // VAO for normal model
    normal_vao: ModelVao,
    // VAO for top layer model
    top_vao: ModelVao,

    // Model data
    steve_model_draw_order_count: i32,
}

impl SkinRenderOpenGL {
    pub fn new(gl: Arc<glow::Context>, is_gles: bool) -> Self {
        unsafe {
            let pg = init_shader(&gl);
            let skin = gl.create_texture().unwrap();
            let cape = gl.create_texture().unwrap();
            let model = ModelVao::new(&gl);
            let top = ModelVao::new(&gl);

            let mut base = BaseSkinRender::new();

            base.info = format!(
                "Renderer: {}\nOpenGL Version: {}\nGLSL Version: {}",
                gl.get_parameter_string(glow::RENDERER),
                gl.get_parameter_string(glow::VERSION),
                gl.get_parameter_string(glow::SHADING_LANGUAGE_VERSION)
            );

            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.enable(CULL_FACE);
            gl.cull_face(BACK);

            Self {
                base: base,
                gl,
                is_gles,
                width: 0,
                height: 0,
                pg,
                texture_skin: skin,
                texture_cape: cape,
                normal_vao: model,
                top_vao: top,
                steve_model_draw_order_count: 0,
            }
        }
    }

    fn draw_cape(&self) {
        if self.base.have_cape && self.base.enable_cape {
            unsafe {
                self.gl.bind_texture(TEXTURE_2D, Some(self.texture_cape));
                let model_loc = self.gl.get_uniform_location(self.pg, "self");
                if let Some(loc) = model_loc {
                    let mat = self.base.get_matrix(ModelPartType::Cape);
                    let mat_slice = std::slice::from_raw_parts(mat.as_ptr(), 16);
                    self.gl
                        .uniform_matrix_4_f32_slice(Some(&loc), false, mat_slice);
                    self.gl
                        .bind_vertex_array(Some(self.normal_vao.cape.vertex_array_object));
                    self.gl.draw_elements(
                        TRIANGLES,
                        self.steve_model_draw_order_count,
                        UNSIGNED_SHORT,
                        0,
                    );
                }
                self.gl.bind_texture(TEXTURE_2D, None);
            }
        }
    }

    unsafe fn draw_skin(&mut self) {
        self.gl
            .bind_texture(glow::TEXTURE_2D, Some(self.texture_skin));

        let model_loc = self.gl.get_uniform_location(self.pg, "self");
        if let Some(loc) = model_loc {
            let model_mat = Mat4::default();
            self.gl
                .uniform_matrix_4_f32_slice(Some(&loc), false, model_mat.as_ref());
            self.gl
                .bind_vertex_array(Some(self.normal_vao.body.vertex_array_object));
            self.gl.draw_elements(
                glow::TRIANGLES,
                self.steve_model_draw_order_count,
                glow::UNSIGNED_SHORT,
                0,
            );

            let model_mat = self.base.get_matrix(ModelPartType::Head);
            self.gl
                .uniform_matrix_4_f32_slice(Some(&loc), false, model_mat.as_ref());
            self.gl
                .bind_vertex_array(Some(self.normal_vao.head.vertex_array_object));
            self.gl.draw_elements(
                glow::TRIANGLES,
                self.steve_model_draw_order_count,
                glow::UNSIGNED_SHORT,
                0,
            );

            let model_mat = self.base.get_matrix(ModelPartType::LeftArm);
            self.gl
                .uniform_matrix_4_f32_slice(Some(&loc), false, model_mat.as_ref());
            self.gl
                .bind_vertex_array(Some(self.normal_vao.left_arm.vertex_array_object));
            self.gl.draw_elements(
                glow::TRIANGLES,
                self.steve_model_draw_order_count,
                glow::UNSIGNED_SHORT,
                0,
            );

            let model_mat = self.base.get_matrix(ModelPartType::RightArm);
            self.gl
                .uniform_matrix_4_f32_slice(Some(&loc), false, model_mat.as_ref());
            self.gl
                .bind_vertex_array(Some(self.normal_vao.right_arm.vertex_array_object));
            self.gl.draw_elements(
                glow::TRIANGLES,
                self.steve_model_draw_order_count,
                glow::UNSIGNED_SHORT,
                0,
            );

            let model_mat = self.base.get_matrix(ModelPartType::LeftLeg);
            self.gl
                .uniform_matrix_4_f32_slice(Some(&loc), false, model_mat.as_ref());
            self.gl
                .bind_vertex_array(Some(self.normal_vao.left_leg.vertex_array_object));
            self.gl.draw_elements(
                glow::TRIANGLES,
                self.steve_model_draw_order_count,
                glow::UNSIGNED_SHORT,
                0,
            );

            let model_mat = self.base.get_matrix(ModelPartType::RightLeg);
            self.gl
                .uniform_matrix_4_f32_slice(Some(&loc), false, model_mat.as_ref());
            self.gl
                .bind_vertex_array(Some(self.normal_vao.right_leg.vertex_array_object));
            self.gl.draw_elements(
                glow::TRIANGLES,
                self.steve_model_draw_order_count,
                glow::UNSIGNED_SHORT,
                0,
            );
        }

        self.gl.bind_texture(glow::TEXTURE_2D, None);
    }

    unsafe fn draw_skin_top(&mut self) {
        self.gl
            .bind_texture(glow::TEXTURE_2D, Some(self.texture_skin));

        let model_loc = self.gl.get_uniform_location(self.pg, "self");
        if let Some(loc) = model_loc {
            let model_mat = self.base.get_matrix(ModelPartType::Body);
            self.gl
                .uniform_matrix_4_f32_slice(Some(&loc), false, model_mat.as_ref());
            self.gl.bind_vertex_array(Some(self.top_vao.body));
            self.gl.draw_elements(
                glow::TRIANGLES,
                self.steve_model_draw_order_count,
                glow::UNSIGNED_SHORT,
                0,
            );

            let model_mat = self.base.get_matrix(ModelPartType::Head);
            self.gl
                .uniform_matrix_4_f32_slice(loc, false, model_mat.as_ref());
            self.gl.bind_vertex_array(Some(self.top_vao.head));
            self.gl.draw_elements(
                glow::TRIANGLES,
                self.steve_model_draw_order_count,
                glow::UNSIGNED_SHORT,
                0,
            );

            let model_mat = self.base.get_matrix(ModelPartType::LeftArm);
            self.gl
                .uniform_matrix_4_f32_slice(loc, false, model_mat.as_ref());
            self.gl.bind_vertex_array(Some(self.top_vao.left_arm));
            self.gl.draw_elements(
                glow::TRIANGLES,
                self.steve_model_draw_order_count,
                glow::UNSIGNED_SHORT,
                0,
            );

            let model_mat = self.base.get_matrix(ModelPartType::RightArm);
            self.gl
                .uniform_matrix_4_f32_slice(loc, false, model_mat.as_ref());
            self.gl.bind_vertex_array(Some(self.top_vao.right_arm));
            self.gl.draw_elements(
                glow::TRIANGLES,
                self.steve_model_draw_order_count,
                glow::UNSIGNED_SHORT,
                0,
            );

            let model_mat = self.base.get_matrix(ModelPartType::LeftLeg);
            self.gl
                .uniform_matrix_4_f32_slice(loc, false, model_mat.as_ref());
            self.gl.bind_vertex_array(Some(self.top_vao.left_leg));
            self.gl.draw_elements(
                glow::TRIANGLES,
                self.steve_model_draw_order_count,
                glow::UNSIGNED_SHORT,
                0,
            );

            let model_mat = self.base.get_matrix(ModelPartType::RightLeg);
            self.gl
                .uniform_matrix_4_f32_slice(loc, false, model_mat.as_ref());
            self.gl.bind_vertex_array(Some(self.top_vao.right_leg));
            self.gl.draw_elements(
                glow::TRIANGLES,
                self.steve_model_draw_order_count,
                glow::UNSIGNED_SHORT,
                0,
            );
        }

        self.gl.bind_texture(glow::TEXTURE_2D, None);
    }

    fn load_model(&mut self) {
        let normal = model::get_steve(self.base.skin_type);
        let top = model::get_steve_top(self.base.skin_type);
        let tex = texture::get_steve_texture(self.base.skin_type);
        let textop = texture::get_steve_texture_top(self.base.skin_type);

        self.steve_model_draw_order_count = normal.head.point.len() as i32;

        put_vao_item(
            &self.gl,
            &self.normal_vao.head,
            &normal.head,
            &tex.head,
            self.pg,
        );
        put_vao_item(
            &self.gl,
            &self.normal_vao.body,
            &normal.body,
            &tex.body,
            self.pg,
        );
        put_vao_item(
            &self.gl,
            &self.normal_vao.left_arm,
            &normal.left_arm,
            &tex.left_arm,
            self.pg,
        );
        put_vao_item(
            &self.gl,
            &self.normal_vao.right_arm,
            &normal.right_arm,
            &tex.right_arm,
            self.pg,
        );
        put_vao_item(
            &self.gl,
            &self.normal_vao.left_leg,
            &normal.left_leg,
            &tex.left_leg,
            self.pg,
        );
        put_vao_item(
            &self.gl,
            &self.normal_vao.right_leg,
            &normal.right_leg,
            &tex.right_leg,
            self.pg,
        );

        put_vao_item(
            &self.gl,
            &self.normal_vao.cape,
            &normal.cape,
            &tex.cape,
            self.pg,
        );

        put_vao_item(
            &self.gl,
            &self.top_vao.head,
            &top.head,
            &textop.head,
            self.pg,
        );
        put_vao_item(
            &self.gl,
            &self.top_vao.body,
            &top.body,
            &textop.body,
            self.pg,
        );
        put_vao_item(
            &self.gl,
            &self.top_vao.left_arm,
            &top.left_arm,
            &textop.left_arm,
            self.pg,
        );
        put_vao_item(
            &self.gl,
            &self.top_vao.right_arm,
            &top.right_arm,
            &textop.right_arm,
            self.pg,
        );
        put_vao_item(
            &self.gl,
            &self.top_vao.left_leg,
            &top.left_leg,
            &textop.left_leg,
            self.pg,
        );
        put_vao_item(
            &self.gl,
            &self.top_vao.right_leg,
            &top.right_leg,
            &textop.right_leg,
            self.pg,
        );
    }

    /// 开始渲染
    pub unsafe fn open_gl_render(&mut self, fb: Option<glow::Framebuffer>) {
        if self.base.switch_skin {
            self.load_skin();
        }
        if self.base.switch_model {
            self.load_model();
        }

        if !self.base.have_skin {
            return;
        }

        if self.base.width == 0 || self.base.height == 0 {
            return;
        }

        if self.base.width != self.width || self.base.height != self.height {
            self.width = self.base.width;
            self.height = self.base.height;
            self.delete_frame_buffer();
            self.init_frame_buffer();
        }

        if self.width == 0 || self.height == 0 {
            return;
        }

        // match self.base.render_type {
        //     SkinRenderType::MSAA => {
        //         self.gl
        //             .bind_framebuffer(glow::FRAMEBUFFER, Some(self.msaa_frame_buffer));
        //     }
        //     SkinRenderType::FXAA => {
        //         self.gl
        //             .bind_framebuffer(glow::FRAMEBUFFER, Some(self.fxaa_frame_buffer));
        //     }
        //     _ => {
        //         self.gl.bind_framebuffer(glow::FRAMEBUFFER, fb);
        //     }
        // }

        self.gl.bind_framebuffer(glow::FRAMEBUFFER, fb);

        self.gl.viewport(0, 0, self.width, self.height);

        // if self.base.render_type == SkinRenderType::FXAA {
        //     self.gl.clear_color(1.0, 1.0, 1.0, 1.0);
        // } else {
        //     self.gl.clear_color(
        //         self.base.back_color.x,
        //         self.base.back_color.y,
        //         self.base.back_color.z,
        //         self.base.back_color.w,
        //     );
        // }

        self.gl.clear_color(1.0, 1.0, 1.0, 1.0);

        self.gl.clear_depth(1.0);
        self.gl
            .clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

        self.check_error();

        self.gl.enable(glow::CULL_FACE);
        self.gl.enable(glow::DEPTH_TEST);
        self.gl.active_texture(glow::TEXTURE0);
        self.gl.use_program(Some(self.pg));

        self.check_error();

        let view_loc = self.gl.get_uniform_location(self.pg, "view");
        let projection_loc = self.gl.get_uniform_location(self.pg, "projection");
        let model_loc = self.gl.get_uniform_location(self.pg, "model");

        let matr = self.base.get_matrix(ModelPartType::Proj);
        if let Some(loc) = projection_loc {
            self.gl
                .uniform_matrix_4_f32_slice(loc, false, matr.as_ref());
        }

        let matr = self.base.get_matrix(ModelPartType::View);
        if let Some(loc) = view_loc {
            self.gl
                .uniform_matrix_4_f32_slice(loc, false, matr.as_ref());
        }

        let matr = self.base.get_matrix(ModelPartType::Model);
        if let Some(loc) = model_loc {
            self.gl
                .uniform_matrix_4_f32_slice(loc, false, matr.as_ref());
        }

        self.check_error();

        self.gl.depth_mask(true);
        self.gl.disable(glow::BLEND);

        self.draw_skin();
        self.draw_cape();

        if self.base.enable_top {
            self.gl.depth_mask(false);
            self.gl.enable(glow::BLEND);
            self.gl.enable(glow::SAMPLE_ALPHA_TO_COVERAGE);
            self.gl
                .blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);

            self.draw_skin_top();

            self.gl.depth_mask(true);
            self.gl.disable(glow::BLEND);
        }

        // MSAA 后处理
        if self.base.render_type == SkinRenderGLType::MSAA {
            self.gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, fb);
            self.gl
                .bind_framebuffer(glow::READ_FRAMEBUFFER, Some(self.msaa_frame_buffer));
            self.gl.blit_framebuffer(
                0,
                0,
                self.width,
                self.height,
                0,
                0,
                self.width,
                self.height,
                glow::COLOR_BUFFER_BIT,
                glow::NEAREST,
            );
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }
        // FXAA 后处理
        else if self.base.render_type == SkinRenderGLType::FXAA {
            self.gl.enable(glow::BLEND);
            self.gl.disable(glow::DEPTH_TEST);
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, fb);
            self.gl.viewport(0, 0, self.width, self.height);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
            self.gl.use_program(Some(self.pg_fxaa));
            self.gl.uniform_2_f32(
                self.fxaa_step,
                1.0 / self.width as f32,
                1.0 / self.height as f32,
            );
            self.gl.active_texture(glow::TEXTURE0);
            self.gl
                .bind_texture(glow::TEXTURE_2D, Some(self.fxaa_texture));
            self.gl.bind_vertex_array(Some(self.fxaa_vao));
            self.gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
            self.gl.bind_vertex_array(None);
            self.gl.enable(glow::DEPTH_TEST);
            self.gl.bind_texture(glow::TEXTURE_2D, None);
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }

        self.check_error();
    }

    /// OpenGL 清理
    pub unsafe fn open_gl_deinit(&mut self) {
        self.base.skin_animation.close();

        // Unbind everything
        self.gl.bind_buffer(glow::ARRAY_BUFFER, None);
        self.gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
        self.gl.bind_vertex_array(None);
        self.gl.use_program(None);

        // Delete all resources
        self.delete_model();
        self.delete_frame_buffer();
        self.delete_texture();
        self.delete_fxaa();

        self.gl.delete_program(self.pg);
        self.gl.delete_program(self.pg_fxaa);
    }

    fn load_skin(&mut self) {
        let base = &mut self.base;

        if base.skin_tex.is_none() {
            base.on_error(ErrorType::InvalidSkin);
            return;
        }

        if base.skin_type == SkinType::Unknown {
            base.on_error(ErrorType::InvalidSkin);
            return;
        }

        let skin_tex = base.skin_tex.as_mut().unwrap();
        load_tex(self.is_gles, &self.gl, skin_tex, self.texture_skin);

        if let Some(cape_tex) = base.cape.as_mut() {
            load_tex(self.is_gles, &self.gl, cape_tex, self.texture_cape);
        }

        base.switch_skin = false;
        base.switch_model = true;
    }
}

impl Drop for SkinRenderOpenGL {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_texture(self.texture_skin);
            self.gl.delete_texture(self.texture_cape);

            self.normal_vao.delete(&self.gl);
            self.top_vao.delete(&self.gl);

            self.gl.delete_program(self.pg);
        }
    }
}

/// 渲染类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkinRenderGLType {
    Normal,
    MSAA,
    FXAA,
}

impl Default for SkinRenderGLType {
    fn default() -> Self {
        Self::Normal
    }
}
