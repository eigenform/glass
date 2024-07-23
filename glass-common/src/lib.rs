
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BayerPattern {
    RGGB,
    BGGR,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelFormat {
    Bayer8(BayerPattern),
    RGBA8,
    RGB8,
}
impl PixelFormat {
    pub fn bytes_per_pixel(&self) -> usize { 
        match self {
            Self::RGB8  => 3,
            Self::RGBA8 => 4,
            Self::Bayer8(_) => 1,
        }
    }
}

/// Container for image data
pub struct PixelData { 
    pub data: Box<[u8]>,
    pub width: usize,
    pub height: usize,
    pub format: PixelFormat,
    pub id: usize,
}
impl PixelData {
    pub fn new(fmt: PixelFormat, width: usize, height: usize) -> Self { 
        let bpp = fmt.bytes_per_pixel();
        let data = vec![0u8; width * height * bpp].into_boxed_slice();
        Self { width, height, data, format: fmt, id: 0 }
    }

    pub fn increment_frame_id(&mut self) {
        self.id = self.id + 1;
    }

    pub fn frame_id(&self) -> usize {
        self.id
    }

    pub fn new_from_slice(
        fmt: PixelFormat, 
        width: usize, 
        height: usize, 
        src: &[u8]
    ) -> Result<Self, &'static str>
    { 
        let mut res = Self::new(fmt, width, height);
        res.fill_from_slice(src)?;
        Ok(res)
    }

    pub fn new_from_file(
        filename: &str, 
        fmt: PixelFormat, 
        width: usize,
        height: usize,
    ) -> Result<Self, &'static str> 
    {
        use std::io::Read;
        let mut f = std::fs::File::open(filename).unwrap();
        let sz = f.metadata().unwrap().len() as usize;
        let mut buf = vec![0; sz];
        f.read_exact(&mut buf).unwrap();
        Self::new_from_slice(fmt, width, height, &buf)
    }


    pub fn fill_from_slice(&mut self, src: &[u8]) -> Result<(), &'static str> {
        if src.len() != self.size_bytes() {
            println!("mismatch source size {} and pixeldata size {}", 
                src.len(), self.size_bytes()
            );
            return Err("Source slice doesn't match PixelData size");
        }
        self.data.copy_from_slice(src);
        Ok(())
    }

    pub fn width(&self) -> usize { 
        self.width 
    }

    pub fn height(&self) -> usize { 
        self.height 
    }

    pub fn format(&self) -> PixelFormat {
        self.format
    }

    pub fn size_bytes(&self) -> usize { 
        self.data.len()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }
}



