

use super::*;

/// Primitives for dealing with control transfers. 
///
/// NOTE: Be aware that the order of 'idx' and val' here are reversed with 
/// respect to the original methods from [rusb] (and with respect to the 
/// actual ordering of fields in a control packet).
impl Mu1603 {
    pub fn ven_read(&mut self, req: u8, idx: u16, val: u16, buf: &mut [u8])
        -> Result<usize, Mu1603Error>
    {
        self.handle.read_control(
            Self::REQ_TYPE_IN, req, val, idx, buf, Self::TIMEOUT
        ).map_err(Mu1603Error::from)
    }

    pub fn ven_write(&mut self, req: u8, idx: u16, val: u16, buf: &[u8])
        -> Result<usize, Mu1603Error>
    {
        self.handle.write_control(
            Self::REQ_TYPE_OUT, req, val, idx, buf, Self::TIMEOUT
        ).map_err(Mu1603Error::from)
    }
}

/// Slightly higher-level helpers for control transfers. 
/// These are the most common interactions while configuring the camera. 
impl Mu1603 {
    pub fn system_cmd(&mut self, idx: u16, val: u16) -> Result<(), Mu1603Error> {
        let mut buf: [u8; 1] = [ 0 ];
        self.ven_read(0x0b, idx, val, &mut buf)?;
        Ok(())
    }

    pub fn sensor_cmd(&mut self, idx: u16, val: u16) -> Result<(), Mu1603Error> {
        let mut buf: [u8; 1] = [ 0 ];
        self.ven_read(0x0b, idx, val, &mut buf)?;
        if buf[0] != 0x08 {
            return Err(Mu1603Error::FailedSensorCmd(idx, val))
        }
        self.ven_read(0x0b, 0x1100, val, &mut buf)?;
        Ok(())
    }
}

/// High-level sets of interactions with the camera. 
impl Mu1603 {

    // 1. Send the "key" `0x0000` to the device. 
    //
    // Most of the requests we use (`0x0a` and `0x0b`) must have their
    // values and indexes XOR'ed with this key before being sent. 
    // We can ignore that requirement after setting this to zero. 
    pub fn set_null_key(&mut self) -> Result<(), Mu1603Error> {
        let mut hbuf: [u8; 2] = [0x00, 0x00];
        self.ven_read(0x16, 0x0000, 0x0000, &mut hbuf)?;
        Ok(())
    }

    // Sequence used to set exposure parameters?
    //
    // It seems like `0x1064` and `0x5000` are the only ones that vary.
    // Not clear how this works yet.
    //
    // - The lowest exposure value [0.244ms] seems to correspond to the 
    //   pair (0x08db, 0x08e3)?
    //
    // - Seems like the lower limit on the value of 0x1064 is 0x000a?
    //  - Changes in increments of 10 or 11?
    //
    // - Seems like the lower limit on the value of 0x5000 is 0x08e3?
    //
    pub fn set_exposure(&mut self, val1064: u16, val5000: u16) 
        -> Result<(), Mu1603Error>
    {
        self.sensor_cmd(0x1063, 0x0000)?;
        self.sensor_cmd(0x1064, val1064)?;
        self.system_cmd(0x4000, 0x0000)?;
        self.system_cmd(0x5000, val5000)?;
        Ok(())
    }

    /// Sequence used to set the analog gain.
    ///
    /// NOTE: Observed values are between 0x610c and 0x61a1. 
    /// Seems like they all start at 0x6000?
    pub fn set_analog_gain(&mut self, val1061: u16) 
        -> Result<(), Mu1603Error> 
    {
        self.sensor_cmd(0x1061, val1061)
    }

    /// Some kind of sensor programming sequence that occurs when changing 
    /// the mode/resolution. 
    ///
    /// NOTE: There are only two commands (for index 0x1004 and 0x1006) that 
    /// vary depending on the requested mode: 
    ///
    /// - Mode 0 => 0x0087, 0x1104
    /// - Mode 1 => 0x0083, 0x11dc
    /// - Mode 2 => 0x0083, 0x11dc
    ///
    /// NOTE: There are apparently timing requirements at certain places in
    /// this sequence.
    ///
    pub fn sensor_program_sequence(&mut self, val_1004: u16, val_1006: u16) 
        -> Result<(), Mu1603Error> 
    {
        self.sensor_cmd(0x1008, 0x4299)?; 
        self.sensor_cmd(0x100f, 0x7fff)?; 
        self.sensor_cmd(0x1001, 0x0030)?; 
        self.sensor_cmd(0x1002, 0x0003)?;
        self.sensor_cmd(0x1003, 0x07e9)?; 
        self.sensor_cmd(0x1000, 0x0003)?; 

        self.sensor_cmd(0x1004, val_1004)?;
        self.sensor_cmd(0x1006, val_1006)?;

        self.sensor_cmd(0x1009, 0x02c0)?; 
        self.sensor_cmd(0x1005, 0x0001)?; 
        self.sensor_cmd(0x1007, 0x7fff)?; 
        self.sensor_cmd(0x100a, 0x0000)?;
        self.sensor_cmd(0x100b, 0x0100)?; 
        self.sensor_cmd(0x100c, 0x0000)?; 
        self.sensor_cmd(0x100d, 0x2090)?; 
        self.sensor_cmd(0x100e, 0x0103)?;
        self.sensor_cmd(0x1010, 0x0000)?; 
        self.sensor_cmd(0x1011, 0x0000)?; 
        std::thread::sleep(Duration::from_millis(5));

        self.sensor_cmd(0x1000, 0x0053)?; 
        self.sensor_cmd(0x1008, 0x0298)?;
        std::thread::sleep(Duration::from_millis(5));

        Ok(())
    }

}
