
mod demosaic;

use eframe::glow;
use eframe::egui;
use glow::HasContext;
use std::sync::{Arc, Mutex, RwLock};


#[repr(C)]
pub struct Vertex {
    /// Position X (range [-1,1])
    pub x: f32,

    /// Position Y (range [-1,1])
    pub y: f32,

    /// Texture X (range [0,1])
    pub u: f32,

    /// Texture Y (range [0,1])
    pub v: f32
}

pub trait GlowProgram {
    type VertexArrayType;

    const VERTICIES: Self::VertexArrayType;

    /// Vertex shader source
    const VERT_SRC: &'static str;

    /// Fragment shader source
    const FRAG_SRC: &'static str;

    fn is_initialized(&self) -> bool;
    fn init(&mut self, gl: &glow::Context) -> Result<(), String>; 
    fn paint(&mut self, gl: &glow::Context);
    fn destroy(&mut self, gl: &glow::Context);
}

//pub trait GlowPainter {
//    fn paint(&mut self, 
//        _info: eframe::egui::PaintCallbackInfo, 
//        painter: &eframe::egui_glow::Painter
//    ); 
//}
//pub trait GlowEguiState {
//    type State: GlowPainter + Send + Sync;
//    fn paint_callback(&self, rect: egui::Rect) -> egui::PaintCallback {
//        egui::PaintCallback {
//            rect, callback: self.glow_callback().clone()
//        }
//    }
//    fn state(&mut self) -> Arc<Mutex<Self::State>>;
//
//    fn destroy(&mut self, gl: &glow::Context);
//
//    fn glow_callback(&self) -> Arc<eframe::egui_glow::CallbackFn> {
//        let state = self.state().clone();
//        Arc::new({eframe::egui_glow::CallbackFn::new(move |info, painter| {
//            state.lock().unwrap().paint(info, painter)
//        })})
//    }
//}

//pub struct PreviewGlow {
//    pub preview: Arc<Mutex<Preview>>,
//    callback: Arc<eframe::egui_glow::CallbackFn>,
//}
//impl PreviewGlow {
//    pub fn new(rgb_data: Arc<RwLock<RgbData>>) -> Self { 
//        let preview = Arc::new(Mutex::new(Preview::new(rgb_data)));
//        let p = preview.clone();
//        let callback = Arc::new({
//            eframe::egui_glow::CallbackFn::new(move |info, painter| 
//            {
//                p.lock().unwrap().paint(info, painter)
//            })
//        });
//
//        Self { 
//            preview,
//            callback,
//        }
//    }
//    pub fn paint(&mut self, rect: egui::Rect) -> egui::PaintCallback {
//        egui::PaintCallback {
//            rect, callback: self.callback.clone()
//        }
//    }
//    pub fn destroy(&mut self, gl: &glow::Context) {
//        self.preview.lock().unwrap().destroy(gl);
//    }
//}



/// Container for boilerplate OpenGL code. 
pub struct GlowHelper;
impl GlowHelper {
    /// Compile and link a program with the provided vertex/fragment shaders. 
    pub unsafe fn compile_and_link(
        gl: &glow::Context,
        vert_src: &str,
        frag_src: &str,
    ) -> Result<glow::Program, String> 
    {
        let (vertex_shader, fragment_shader) = {
            let vertex_shader = gl.create_shader(glow::VERTEX_SHADER)?;
            gl.shader_source(vertex_shader, vert_src);
            gl.compile_shader(vertex_shader);
            if !gl.get_shader_compile_status(vertex_shader) {
                return Err(gl.get_shader_info_log(vertex_shader));
            }

            let fragment_shader = gl.create_shader(glow::FRAGMENT_SHADER)?;
            gl.shader_source(fragment_shader, frag_src);
            gl.compile_shader(fragment_shader);
            if !gl.get_shader_compile_status(fragment_shader) {
                return Err(gl.get_shader_info_log(fragment_shader));
            }
            (vertex_shader, fragment_shader)
        };

        let program = gl.create_program()?;
        gl.attach_shader(program, vertex_shader);
        gl.attach_shader(program, fragment_shader);
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            return Err(gl.get_program_info_log(program));
        }

        // You can apparently delete these after linking
        gl.detach_shader(program, vertex_shader);
        gl.detach_shader(program, fragment_shader);
        gl.delete_shader(vertex_shader);
        gl.delete_shader(fragment_shader);

        Ok(program)
    }

    pub unsafe fn allocate_bind_texture(
        gl: &glow::Context, 
        fmt: PixelFormat, 
        height: usize, 
        width: usize,
    ) -> Result<glow::Texture, String>
    {
        let format = match fmt {
            PixelFormat::RGB8      => glow::RGB,
            PixelFormat::Bayer8(_) => glow::RED,
            PixelFormat::RGBA8     => glow::RGBA,
        };

        let texture = gl.create_texture()?;
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            format as _,
            height as i32,
            width as i32,
            0, 
            format as _,
            glow::UNSIGNED_BYTE,
            None
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_S,
            glow::CLAMP_TO_EDGE as _,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_T,
            glow::CLAMP_TO_EDGE as _,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::NEAREST as _,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::NEAREST as _,
        );
        Ok(texture)
    }
}



pub struct Preview {
    program: Option<glow::Program>,
    buffer: Option<glow::Buffer>,
    vertex_array: Option<glow::VertexArray>,
    texture: Option<glow::Texture>,

    pub rgb_data: Arc<RwLock<PixelData>>,
    pub initialized: bool,
}
impl Preview {

    const VERT: &'static str = r#"
        #version 150 core
        in vec2 in_position;
        in vec2 in_texcoord;
        out vec2 frag_texcoord;
        void main() {
            // The position of this vertex
            gl_Position = vec4(in_position, 0.0, 1.0);

            // The texture coordinate for this vertex
            frag_texcoord = in_texcoord;
        }
    "#;

    const FRAG: &'static str = r#"
        #version 150 core
        uniform sampler2D tex;
        in vec2 frag_texcoord;
        out vec4 out_color;
        void main() {

            // Get the RGB colors of this pixel from the texture
            vec4 col = texture(tex, frag_texcoord);

            // Convert to RGBA
            out_color = vec4(col.rgb, 1.0);
        }
    "#;

    const VERTICIES: [Vertex; 6] = [
        // (The first triangle)
        Vertex { x: -1.0, y:  1.0, u: 0.0, v: 1.0 }, // top-left
        Vertex { x:  1.0, y:  1.0, u: 1.0, v: 1.0 }, // top-right
        Vertex { x: -1.0, y: -1.0, u: 0.0, v: 0.0 }, // bottom-left

        // (The second triangle)
        Vertex { x:  1.0, y:  1.0, u: 1.0, v: 1.0 }, // top-right
        Vertex { x:  1.0, y: -1.0, u: 1.0, v: 0.0 }, // bottom-right
        Vertex { x: -1.0, y: -1.0, u: 0.0, v: 0.0 }, // bottom-left
    ];
}

impl Preview {
    pub fn new(rgb_data: Arc<RwLock<PixelData>>) -> Self { 
        Self { 
            program: None,
            buffer: None,
            vertex_array: None,
            texture: None,
            rgb_data,
            initialized: false,
        }
    }

    pub fn destroy(&mut self, gl: &glow::Context) {
        if let Some(program) = self.program.take() {
            unsafe { gl.delete_program(program) };
        }
        if let Some(buffer) = self.buffer.take() {
            unsafe { gl.delete_buffer(buffer) };
        }
        if let Some(vertex_array) = self.vertex_array.take() {
            unsafe { gl.delete_vertex_array(vertex_array) };
        }
        if let Some(texture) = self.texture.take() {
            unsafe { gl.delete_texture(texture) };
        }
        self.initialized = false;
    }

    pub fn paint(&mut self, gl: &glow::Context) 
    {
        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, self.buffer);
            gl.bind_vertex_array(self.vertex_array);
            gl.use_program(self.program);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, self.texture);

            let rgb_data = self.rgb_data.read().unwrap();
            gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                0,
                0,
                2320,
                1740,
                glow::RGB,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(&rgb_data.data)
            );
            drop(rgb_data);
            gl.draw_arrays(glow::TRIANGLES, 0, 6);
        }
    }

    pub fn init(&mut self, gl: &glow::Context) -> Result<(), String> {
        let program = unsafe { 
            GlowHelper::compile_and_link(&gl, Self::VERT, Self::FRAG)
        }?;
        self.program = Some(program);

        unsafe { 
            let buffer = gl.create_buffer()?;
            let slice: &[u8] = std::slice::from_raw_parts(
                Self::VERTICIES.as_ptr() as *const u8, 
                std::mem::size_of_val(&Self::VERTICIES)
            );
            self.buffer = Some(buffer);
            gl.bind_buffer(glow::ARRAY_BUFFER, self.buffer);
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER, slice, glow::STATIC_DRAW
            );
        }

        // Define the layout of the vertex buffer 
        unsafe { 
            let vertex_array = gl.create_vertex_array()?;
            self.vertex_array = Some(vertex_array);
            gl.bind_vertex_array(self.vertex_array);
            let sz_float = std::mem::size_of::<f32>() as i32;

            let pos = gl.get_attrib_location(program, "in_position")
                .expect("no in_position attribute");
            gl.vertex_attrib_pointer_f32(
                pos, 2, glow::FLOAT, false, 
                4 * sz_float,  // stride
                0 * sz_float,  // offset
            );
            gl.enable_vertex_attrib_array(pos);

            let tex = gl.get_attrib_location(program, "in_texcoord")
                .expect("no in_texcoord attribute");
            gl.vertex_attrib_pointer_f32(
                tex, 2, glow::FLOAT, false,
                4 * sz_float,  // stride
                2 * sz_float,  // offset
            );
            gl.enable_vertex_attrib_array(tex);
        }

        unsafe { 
            let texture = gl.create_texture()?;
            self.texture = Some(texture);
            gl.bind_texture(glow::TEXTURE_2D, self.texture);

            let rgb_data = self.rgb_data.read().unwrap();
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGB8 as _,
                2320,
                1740,
                0,
                glow::RGB,
                glow::UNSIGNED_BYTE,
                Some(&rgb_data.data)
            );
            drop(rgb_data);

            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as _,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as _,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as _,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as _,
            );
        }

        self.initialized = true;

        Ok(())
    }
}


