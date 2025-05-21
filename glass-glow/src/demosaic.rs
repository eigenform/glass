
use glow;
use glow::HasContext;
use crate::*;
use std::sync::{Arc, RwLock};
use crate::PixelData;

/// OpenGL program used to recover an RGB image from raw sensor data. 
///
/// As far as I can tell, the situation is something like this: 
///
/// - Your image sensor has a "Bayer filter", where each pixel of the 
///   output only represents the intensity of *a single color*
/// - The pattern on your sensor is probably "RGGB" 
/// - We have to use some kind of algorithm (a "debayering" or "demosaicing" 
///   algorithm) to recover the full RGB values for each pixel
///
/// Since we want to have a responsive "preview" of the image from the sensor,
/// we want to do this on GPU (and avoid using the CPU because it's slow). 
/// On top of that, we also want the ability to read the resulting image 
/// back into RAM (in order to "acquire" images and save them to disk). 
///
/// We need the process to look something like this: 
///
/// - Upload the raw bayer image from RAM to the GPU
/// - Apply the shader to the image
/// - Draw the resulting image to the default framebuffer (the preview window)
/// - Read the resulting image back to RAM
///
pub struct DemosaicQuad {
    program: Option<glow::Program>,
    vao: Option<glow::VertexArray>,
    vbo: Option<glow::Buffer>,

    /// Input texture (bayer)
    pub input_texture: Option<glow::Texture>,

    /// Output texture (RGB)
    pub output_texture: Option<glow::Texture>,

    // Intermediate framebuffer for the resulting image (?)
    fbo: Option<glow::Framebuffer>,

    capture: Arc<RwLock<PixelData>>,

    width: usize,
    height: usize,
    initialized: bool,
}
impl DemosaicQuad {
    pub fn new(width: usize, height: usize, capture: Arc<RwLock<PixelData>>) -> Self { 
        Self { 
            program: None,
            capture,
            vao: None,
            vbo: None,
            fbo: None,
            input_texture: None,
            output_texture: None,
            width, 
            height,
            initialized: false,
        }
    }

    /// Upload data to the input texture. 
    ///
    /// FIXME: We aren't validating the size of 'data' right now ...
    pub fn update_input_texture(&mut self, gl: &glow::Context, data: &[u8]) {
        if !self.is_initialized() {
            return;
        }
        unsafe { 
            gl.use_program(self.program);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, self.input_texture);
            gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                0,
                0,
                self.width as i32,
                self.height as i32,
                glow::LUMINANCE,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(data)
            );
        }
    }

    pub fn paint_to_fbo(&mut self, gl: &glow::Context, data: &[u8]) {
        unsafe { 
            gl.use_program(self.program);

            // Switch to our intermediate framebuffer
            gl.bind_framebuffer(glow::FRAMEBUFFER, self.fbo);

            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            gl.bind_vertex_array(self.vao);
            gl.bind_buffer(glow::ARRAY_BUFFER, self.vbo);

            // top-to-bottom, left-to-right
            //const VERTICIES: [Vertex; 4] = [
            //    Vertex { x: -1.0, y:  1.0, u: 0.0, v: 0.0 },
            //    Vertex { x:  1.0, y:  1.0, u: 1.0, v: 0.0 },
            //    Vertex { x: -1.0, y: -1.0, u: 0.0, v: 1.0 },
            //    Vertex { x:  1.0, y: -1.0, u: 1.0, v: 1.0 },
            //];
            //let slice: &[u8] = std::slice::from_raw_parts(
            //    Self::VERTICIES.as_ptr() as *const u8, 
            //    std::mem::size_of_val(&Self::VERTICIES)
            //);
            //gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, slice, glow::STATIC_DRAW);
            //let position = gl.get_attrib_location(self.program.unwrap(), "position").expect("");
            //gl.vertex_attrib_pointer_f32(
            //    position, 2, glow::FLOAT, false,
            //    std::mem::size_of::<Vertex>() as i32,
            //    (std::mem::size_of::<f32>() * 0) as i32,
            //);
            //let tex_coord = gl.get_attrib_location(self.program.unwrap(), "tex_coords").expect("");
            //gl.vertex_attrib_pointer_f32(
            //    tex_coord, 2, glow::FLOAT, false,
            //    std::mem::size_of::<Vertex>() as i32,
            //    (std::mem::size_of::<f32>() * 2) as i32,
            //);
            //gl.enable_vertex_attrib_array(position);
            //gl.enable_vertex_attrib_array(tex_coord);


            // Bind input texture to texture unit 1
            gl.active_texture(glow::TEXTURE1);
            gl.bind_texture(glow::TEXTURE_2D, self.input_texture);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as _);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as _);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as _);
            gl.tex_parameter_i32( glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as _);

            // Attach output texture to framebuffer
            gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D, self.output_texture, 0
            );

            gl.draw_buffers(&[glow::COLOR_ATTACHMENT0]);
            if gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
                println!("framebuffer incomplete?");
                return;
            }

            // Tell the fragment shader we're sampling from texture unit 1
            let u_texture = gl.get_uniform_location(self.program.unwrap(), "source").expect("");
            gl.uniform_1_i32(Some(&u_texture), 1);

            gl.viewport(0, 0, self.width as _ , self.height as _ );
            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);

            if let Ok(mut data) = self.capture.write() {
                println!("reading fbo");
                gl.bind_texture(glow::TEXTURE_2D, self.output_texture);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as _);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as _);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as _);
                gl.tex_parameter_i32( glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as _);

                gl.get_tex_image(glow::TEXTURE_2D,
                    0, 
                    glow::RGB, 
                    glow::UNSIGNED_BYTE,
                    glow::PixelPackData::Slice(&mut data.data)
                );

                //gl.read_pixels(0, 0,
                //    (self.width as i32), 
                //     (self.height as i32), 
                //    glow::RGB, 
                //    glow::UNSIGNED_BYTE,
                //    glow::PixelPackData::Slice(&mut data.data)
                //);
                data.increment_frame_id();
            }

            // Return to rendering in the default framebuffer
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }
    }



}
impl GlowProgram for DemosaicQuad {

    type VertexArrayType = [Vertex; 4];

    const VERTICIES: Self::VertexArrayType = [
        Vertex { x: -1.0, y:  1.0, u: 0.0, v: 0.0 },
        Vertex { x:  1.0, y:  1.0, u: 1.0, v: 0.0 },
        Vertex { x: -1.0, y: -1.0, u: 0.0, v: 1.0 },
        Vertex { x:  1.0, y: -1.0, u: 1.0, v: 1.0 },
    ];

    const VERT_SRC: &'static str = include_str!("demosaic_vert.glsl");
    const FRAG_SRC: &'static str = include_str!("demosaic_frag.glsl");

    fn is_initialized(&self) -> bool { self.initialized }

    // NOTE: This sensor is apparently BGGR (red component at [1,1]).
    // This is configured with the 'firstRed' uniform. 
    // Some other patterns for reference: 
    //
    // RGGB | GRGB | BGRG | BGGR
    // -----+------+------+------
    //  R G | G R  | B G  | B G
    //  G B | G B  | R G  | G R
    // -----+------+------+------
    // [0,0]| [0,1]| [1,0]| [1,1]
    //
    fn init(&mut self, gl: &glow::Context) -> Result<(), String> {
        let program = unsafe { 
            GlowHelper::compile_and_link(gl, Self::VERT_SRC, Self::FRAG_SRC)
        }?;
        self.program = Some(program);
        unsafe { 
            gl.use_program(self.program);

            println!("DRAW_FRAMEBUFFER_BINDING = {}", 
                gl.get_parameter_i32(glow::DRAW_FRAMEBUFFER_BINDING)
            );
            println!("READ_FRAMEBUFFER_BINDING = {}", 
                gl.get_parameter_i32(glow::READ_FRAMEBUFFER_BINDING)
            );

            // Allocate new framebuffer.
            // Allocate output texture [to-be-attached to framebuffer].

            let framebuffer = gl.create_framebuffer()?;
            println!("fbo = {}", framebuffer.0);
            self.fbo = Some(framebuffer);
            let output_texture = GlowHelper::allocate_bind_texture(&gl,
                PixelFormat::RGB8,
                self.width, self.height
            )?;
            self.output_texture = Some(output_texture);

            // Allocate buffer for vertex data

            let vao = gl.create_vertex_array()?;
            gl.bind_vertex_array(Some(vao));
            self.vao = Some(vao);
            let vbo = gl.create_buffer()?;
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            self.vbo = Some(vbo);

            // Upload vertex data

            let slice: &[u8] = std::slice::from_raw_parts(
                Self::VERTICIES.as_ptr() as *const u8, 
                std::mem::size_of_val(&Self::VERTICIES)
            );
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, slice, glow::STATIC_DRAW);

            // Specify the format of vertex data. 
            // See Self::VERTICIES

            let position = gl.get_attrib_location(program, "position").expect("");
            gl.vertex_attrib_pointer_f32(
                position, 2, glow::FLOAT, false,
                std::mem::size_of::<Vertex>() as i32,
                (std::mem::size_of::<f32>() * 0) as i32,
            );
            let tex_coord = gl.get_attrib_location(program, "tex_coords").expect("");
            gl.vertex_attrib_pointer_f32(
                tex_coord, 2, glow::FLOAT, false,
                std::mem::size_of::<Vertex>() as i32,
                (std::mem::size_of::<f32>() * 2) as i32,
            );
            gl.enable_vertex_attrib_array(position);
            gl.enable_vertex_attrib_array(tex_coord);

            // Create a texture for the input data

            let input_texture = GlowHelper::allocate_bind_texture(&gl,
                PixelFormat::Bayer8(BayerPattern::BGGR), 
                self.width, self.height
            )?;
            self.input_texture = Some(input_texture);

            // Upload uniforms expected by the shader

            let u_firstred = gl.get_uniform_location(
                self.program.unwrap(), "firstRed"
            ).expect("");
            gl.uniform_2_f32(Some(&u_firstred), 1.0, 1.0);

            let u_sourcesize = gl.get_uniform_location(
                self.program.unwrap(), "sourceSize"
            ).expect("");
            gl.uniform_4_f32(Some(&u_sourcesize), 
                self.width as f32,
                self.height as f32,
                1.0 / self.width as f32,
                1.0 / self.height as f32,
            );
        }

        self.initialized = true;

        Ok(())
    }

    // Render to the default framebuffer. 
    fn paint(&mut self, gl: &glow::Context) {
        unsafe { 
            gl.use_program(self.program);

            gl.bind_framebuffer(glow::FRAMEBUFFER, None);

            gl.bind_vertex_array(self.vao);
            gl.bind_buffer(glow::ARRAY_BUFFER, self.vbo);

            // The raw input texture is bound to TEXTURE0
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, self.input_texture);

            // Tell the shader that the source texture is TEXTURE0 (?)
            let u_texture = gl.get_uniform_location(
                self.program.unwrap(), "source"
            ).expect("");
            gl.uniform_1_i32(Some(&u_texture), 0);

            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        }
    }

    fn destroy(&mut self, gl: &glow::Context) {
        unsafe { 
            if let Some(vbo) = self.vbo {
                gl.delete_buffer(vbo);
            }
            if let Some(vao) = self.vao {
                gl.delete_vertex_array(vao);
            }
            if let Some(input_texture) = self.input_texture {
                gl.delete_texture(input_texture);
            }
            if let Some(program) = self.program {
                gl.delete_program(program);
            }
        }
        self.vbo = None;
        self.vao = None;
        self.input_texture = None;
        self.program = None;
    }
}



