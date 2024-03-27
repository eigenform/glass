
pub mod demosaic;
pub use crate::demosaic::*;

use glow;
use glow::HasContext;
use glass_common::*;

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
    const VERT_SRC: &'static str;
    const FRAG_SRC: &'static str;

    fn init(&mut self, gl: &glow::Context) -> Result<(), String>; 
    fn is_initialized(&self) -> bool;
    fn paint(&mut self, gl: &glow::Context);
    fn destroy(&mut self, gl: &glow::Context);
}


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

    /// Allocate and bind a texture.
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


