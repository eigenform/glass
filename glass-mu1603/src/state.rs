
// NOTE: The number of lines per frame should coincide with the
// number of vsync pulses. 
//
// NOTE: The number of hsync pulses per line coincides with the exposure time. 
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Mu1603Mode { 
    /// Resolution: 4632x3488
    Mode0, 
    /// Resolution: 2320x1740
    Mode1, 
    /// Resolution: 1536x1160
    Mode2
}
impl Mu1603Mode {
    pub fn description(&self) -> &'static str {
        match self { 
            Self::Mode0 => "4632x3488",
            Self::Mode1 => "2320x1740",
            Self::Mode2 => "1536x1160",
        }

    }
    pub fn max_hsync(&self) -> u16 { 
        match self { 
            Self::Mode0 => 0x0e24, // 3620
            Self::Mode1 => 0x08e3, // 2275
            Self::Mode2 => 0x04ca, // 1226
        }
    }
    pub fn width(&self) -> usize { 
        match self { 
            Self::Mode0 => 4632,
            Self::Mode1 => 2320,
            Self::Mode2 => 1536,
        }
    }
    pub fn height(&self) -> usize { 
        match self { 
            Self::Mode0 => 3488,
            Self::Mode1 => 1740,
            Self::Mode2 => 1160,
        }
    }
    pub fn dimensions(&self) -> (usize, usize) { 
        (self.width(), self.height())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Mu1603BitDepth {
    Depth8,
    Depth12,
}
impl Mu1603BitDepth {
    pub fn bpp(&self) -> usize { 
        match self { 
            Self::Depth8 => 1,
            Self::Depth12 => 2,
        }
    }
}

/// The exposure time [in microseconds].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ExposureTime(usize);
impl ExposureTime {
    pub const MIN: usize = 001_000;
    pub const MAX: usize = 125_000;
    pub const DEFAULT: usize = 94_000;

    pub fn new_from_ms(ms: usize) -> Self { 
        let res = (ms * 1000).clamp(Self::MIN, Self::MAX);
        Self(res)
    }
    pub fn new_from_us(us: usize) -> Self { 
        let res = us.clamp(Self::MIN, Self::MAX);
        Self(res)
    }


    pub fn value(&self) -> usize { self.0 }
    pub fn value_mut(&mut self) -> &mut usize { &mut self.0 }

    pub fn microseconds(&self) -> usize { 
        self.0
    }
    pub fn milliseconds(&self) -> usize { 
        self.0 / 1000
    }


    pub fn convert(&self, mode: Mu1603Mode, num_lines: u16) 
        -> Option<(u16, u16, u16)>
    {
        pub const CYCLES_PER_US:       usize = 54;
        pub const MIN_CYCLES_PER_LINE: usize = 10;
        let req_exposure_us  = self.0;
        let lines            = num_lines as usize;
        let cycles_per_hsync = num_lines as usize;

        // ([us] * [cycles/us]) = [cycles]
        let req_exposure_cycles = req_exposure_us * CYCLES_PER_US;
        // [cycles] / [lines] = [cycles/line]
        let req_cycles_per_line = (req_exposure_cycles / lines) as usize;

        // NOTE: I think these values correspond to the maximum number of 
        // hsync strobes for a line. If we were using this value, we'd be 
        // triggering the shutter for each pixel in a line, meaning that the 
        // exposure time is minimal (as fast as possible).
        let mut hsync_per_vsync = mode.max_hsync() as usize;

        let mut eff_cycles_per_line = MIN_CYCLES_PER_LINE;

        if req_cycles_per_line < (hsync_per_vsync - MIN_CYCLES_PER_LINE) {
            eff_cycles_per_line = hsync_per_vsync - req_cycles_per_line;
        } 
        else {
            hsync_per_vsync = req_cycles_per_line + MIN_CYCLES_PER_LINE;
            if eff_cycles_per_line >= 0x1fff4 {
                hsync_per_vsync = 0x1ffff;
            }
        }

        let res_1064 = (eff_cycles_per_line & 0x1fff) as u16;
        let res_4000 = ((hsync_per_vsync >> 16) & 0xffff) as u16;
        let res_5000 = (hsync_per_vsync & 0xffff) as u16;
        Some((res_1064, res_4000, res_5000))
    }
}
impl Default for ExposureTime {
    fn default() -> Self { Self(Self::DEFAULT) }
}


#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AnalogGain(usize);
impl AnalogGain {
    pub const MIN: usize = 100;
    pub const MAX: usize = 300;
    pub const DEFAULT: usize = 100;
    //pub const MIN: u16 = 0x610c;
    //pub const MAX: u16 = 0x610c;
    //pub const DEFAULT: u16 = 0x610c;

    pub fn new_from_percent(percent: usize) -> Self {
        let res = percent.clamp(Self::MIN, Self::MAX);
        Self(res)
    }
    pub fn new_from_u16(val: u16) -> Self { 
        unimplemented!();
    }

    pub fn value(&self) -> usize { self.0 }
    pub fn value_mut(&mut self) -> &mut usize { &mut self.0 }
    pub fn percent(&self) -> usize { 
        self.0
    }

}
impl Default for AnalogGain {
    fn default() -> Self { Self(Self::DEFAULT) }
}

/// Reflecting the current state of the camera. 
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Mu1603State {
    pub id: usize,
    pub mode: Mu1603Mode,
    pub exposure: ExposureTime,
    pub analog_gain: AnalogGain,
    pub bitdepth: Mu1603BitDepth,
}

impl Mu1603State {
    pub fn exposure_ms(&self) -> usize { 
        self.exposure.milliseconds()
    }
    pub fn analog_gain_percent(&self) -> usize { 
        self.analog_gain.percent()
    }


    pub fn mode(&self) -> &Mu1603Mode {
        &self.mode
    }
    pub fn exposure(&self) -> &ExposureTime {
        &self.exposure
    }
    pub fn analog_gain(&self) -> &AnalogGain {
        &self.analog_gain
    }
    pub fn bitdepth(&self) -> &Mu1603BitDepth {
        &self.bitdepth
    }

    pub fn mode_mut(&mut self) -> &mut Mu1603Mode {
        &mut self.mode
    }
    pub fn exposure_mut(&mut self) -> &mut ExposureTime {
        &mut self.exposure
    }
    pub fn analog_gain_mut(&mut self) -> &mut AnalogGain {
        &mut self.analog_gain
    }
    pub fn bitdepth_mut(&mut self) -> &mut Mu1603BitDepth { 
        &mut self.bitdepth
    }

}






