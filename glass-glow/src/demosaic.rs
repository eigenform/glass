
use glow;
use glow::HasContext;
use crate::*;

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
/// - Read the resulting image back to RAM
///
pub struct DemosaicQuad {
    program: Option<glow::Program>,
    vao: Option<glow::VertexArray>,
    vbo: Option<glow::Buffer>,
    pub texture: Option<glow::Texture>,

    width: usize,
    height: usize,
    initialized: bool,
}
impl DemosaicQuad {
    pub fn new(width: usize, height: usize) -> Self { 
        Self { 
            program: None,
            vao: None,
            vbo: None,
            texture: None,
            width, height,
            initialized: false,
        }
    }

    pub fn update_texture(&mut self, gl: &glow::Context, data: &[u8]) {
        if !self.is_initialized() {
            return;
        }

        unsafe { 
            gl.use_program(self.program);
            gl.bind_texture(glow::TEXTURE_2D, self.texture);
            gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                0,
                0,
                self.width as i32,
                self.height as i32,
                glow::RED,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(data)
            );

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

            let vao = gl.create_vertex_array()?;
            gl.bind_vertex_array(Some(vao));
            self.vao = Some(vao);

            let vbo = gl.create_buffer()?;
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            self.vbo = Some(vbo);
            let slice: &[u8] = std::slice::from_raw_parts(
                Self::VERTICIES.as_ptr() as *const u8, 
                std::mem::size_of_val(&Self::VERTICIES)
            );
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, slice, glow::STATIC_DRAW);

            let position = gl.get_attrib_location(program, "position")
                .expect("");
            gl.vertex_attrib_pointer_f32(
                position, 2, glow::FLOAT, false, 
                std::mem::size_of::<Vertex>() as i32,
                (std::mem::size_of::<f32>() * 0) as i32,
            );
            let tex_coord = gl.get_attrib_location(program, "tex_coords")
                .expect("");
            gl.vertex_attrib_pointer_f32(
                tex_coord, 2, glow::FLOAT, false,
                std::mem::size_of::<Vertex>() as i32,
                (std::mem::size_of::<f32>() * 2) as i32,
            );
            gl.enable_vertex_attrib_array(position);
            gl.enable_vertex_attrib_array(tex_coord);

            let texture = GlowHelper::allocate_bind_texture(&gl,
                PixelFormat::Bayer8(BayerPattern::BGGR), 
                self.width, self.height
            )?;
            self.texture = Some(texture);

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

    fn paint(&mut self, gl: &glow::Context) {
        unsafe { 
            gl.use_program(self.program);
            gl.bind_vertex_array(self.vao);
            gl.bind_buffer(glow::ARRAY_BUFFER, self.vbo);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_RECTANGLE, self.texture);

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
            if let Some(texture) = self.texture {
                gl.delete_texture(texture);
            }
            if let Some(program) = self.program {
                gl.delete_program(program);
            }
        }
        self.vbo = None;
        self.vao = None;
        self.texture = None;
        self.program = None;
    }
}



