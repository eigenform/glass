
mod usb;
mod state;

pub use state::*;

use pretty_hex::*;
use std::time::Duration;
use rusb::{ 
    Context, UsbContext, Device, DeviceHandle, DeviceDescriptor,
    request_type, Direction, RequestType, Recipient,
};

#[derive(Debug)]
pub enum Mu1603Error { 
    Rusb(rusb::Error),
    FirstFrame,
    Unimplemented,
    NotStreaming,
    FailedSensorCmd(u16, u16),
}
impl From<rusb::Error> for Mu1603Error {
    fn from(e: rusb::Error) -> Self { Self::Rusb(e) }
}


pub struct Mu1603 {
    /// libusb handle to the device
    handle: DeviceHandle<Context>,
    state: Option<Mu1603State>,
    prev_state: Option<Mu1603State>,
}
impl Mu1603 {
    /// USB Vendor ID
    pub const VID: u16 = 0x0547;

    /// USB Product ID
    pub const PID: u16 = 0x3016;

    /// Default timeout for USB control transfers
    pub const TIMEOUT: Duration = Duration::from_secs(5);

    /// Default mode for initialization
    pub const DEFAULT_MODE: Mu1603Mode = Mu1603Mode::Mode1;

    /// Input vendor request type
    pub const REQ_TYPE_IN: u8 = request_type(
        Direction::In, RequestType::Vendor, Recipient::Device
    );
    /// Output vendor request type
    pub const REQ_TYPE_OUT: u8 = request_type(
        Direction::Out, RequestType::Vendor, Recipient::Device
    );

    pub fn state(&self) -> Option<Mu1603State> {
        self.state
    }
    pub fn is_streaming(&self) -> bool {
        self.state.is_some()
    }

    /// Try to obtain a handle to the camera. 
    pub fn try_open(ctx: &mut Context) -> rusb::Result<Self> {
        let res = ctx.open_device_with_vid_pid(Self::VID, Self::PID);
        if let Some(mut handle) = res {
            if let Ok(true) = handle.kernel_driver_active(0) {
                handle.detach_kernel_driver(0)?;
            }
            handle.set_active_configuration(1)?;
            handle.claim_interface(0)?;
            Ok(Self { 
                handle, 
                state: None,
                prev_state: None,
            })
        } else { 
            Err(rusb::Error::NoDevice)
        }
    }
}

impl Mu1603 {
    pub fn apply_state(&mut self, next_state: Mu1603State) 
    {
        if let Some(this_state) = self.state {
            if this_state.mode() != next_state.mode() {
            }
            if this_state.exposure() != next_state.exposure() {
            }
            if this_state.analog_gain() != next_state.analog_gain() {
            }
        } 
        else {
        }

        self.state = Some(next_state);
    }

    // self.sys_write(0x0200, 0x0001)?; // 12-bit depth?
    // self.sys_write(0x8000, 0x09b0)?;
    // self.set_exposure(0x0637, 0x0e24)?;

    // self.sensor_mode_init(0x0087, 0x1104)?;

    // // -------
    // self.sys_write(0x1200, 0x0001)?;
    // std::thread::sleep(Duration::from_millis(20)); // should be 20?
    // self.sys_write(0x2000, 0x0000)?;
    // self.sys_write(0x1200, 0x0002)?;
    // std::thread::sleep(Duration::from_millis(20)); // should be 20?

    // self.sys_write(0x0200, 0x0001)?; // '0x0001' enables 12-bit depth?
    // self.sys_write(0x0a00, 0x0001)?;
    // std::thread::sleep(Duration::from_millis(20)); // should be 20?
    // self.sys_write(0x0a00, 0x0000)?;
    // std::thread::sleep(Duration::from_millis(20)); // should be 20?

    // self.sensor_mode_init(0x0083, 0x11dc)?;

    // // -------
    // self.sys_write(0x103b, 0x0000)?;

    // self.sys_write(0x2000, 0x0001)?; // related to mode 1
    // self.sys_write(0x1200, 0x0003)?; // related to mode 1
    // std::thread::sleep(Duration::from_millis(10));

    // // Perhaps resolution related?
    // self.sys_write(0x8000, 0x060c)?; // related to mode 1?

    // // mode 1,  94000us - 0x000a, 0x0cbd
    // // mode 1, 150000us - 0x000a, 0x144e
    // self.set_exposure(0x000a, 0x0cbd)?;

    // self.sys_write(0x0a00, 0x0001)?;
    // //std::thread::sleep(Duration::from_millis(10));

    // self.set_exposure(0x000a, 0x0cbd)?;
    // self.set_analog_gain(0x610c)?;



    pub fn start_stream(&mut self, init_mode: Mu1603Mode) 
        -> Result<(), Mu1603Error>
    {
        if self.is_streaming() {
            return Ok(());
        }

        // 1. Send the key `0x0000` to the device. 
        //
        // NOTE: There's some kind of challenge-response handshake that occurs 
        // after this (according to 'drivers/media/usb/gspca/touptek.c') in my 
        // packet captures, but we can apparently just ignore it? 
        self.set_null_key()?;

        // 2. I have no idea what this does.
        // Probably related to enabling the sensor. 
        self.ven_write(0x01, 0x000f, 0x0001, &[])?;
        self.ven_write(0x01, 0x000f, 0x0000, &[])?;
        self.ven_write(0x01, 0x000f, 0x0001, &[])?;

        // 3. I have no idea what this does. 
        let mut hbuf: [u8; 2] = [0x00, 0x00];
        self.ven_read(0x0a, 0xffff, 0x0000, &mut hbuf)?;
        self.ven_read(0x0a, 0xffff, 0x0000, &mut hbuf)?;
        self.ven_read(0x0a, 0xfeff, 0x0000, &mut hbuf)?;
        self.ven_read(0x0a, 0xfeff, 0x0000, &mut hbuf)?;

        // 4. Do some fixed initialization sequence. 
        // 
        // NOTE: It seems like this starts the sensor in mode 0.
        self.sensor_program_sequence(0x0087, 0x1104)?;

        self.system_cmd(0x1200, 0x0001)?;
        std::thread::sleep(Duration::from_millis(20));
        self.system_cmd(0x2000, 0x0000)?;
        self.system_cmd(0x1200, 0x0002)?;
        std::thread::sleep(Duration::from_millis(20));

        self.system_cmd(0x0200, 0x0000)?; // Bit-depth?
        self.system_cmd(0x0a00, 0x0001)?;
        self.system_cmd(0x0a00, 0x0000)?;
        std::thread::sleep(Duration::from_millis(20));

        // 6. Setup the requested mode/resolution? 
        //
        // NOTE: The value for 0x8000 seems to depend on the mode and 
        // something else (maybe the bitdepth?)  
        //
        // NOTE: This is sensitive to timing; the 10ms sleep is *required*.
        match init_mode { 
            Mu1603Mode::Mode0 => {
                self.sensor_program_sequence(0x0087, 0x1104)?;
                self.sensor_cmd(0x103b, 0x0000)?;
                self.system_cmd(0x2000, 0x0000)?;
                self.system_cmd(0x1200, 0x0002)?;
                std::thread::sleep(Duration::from_millis(10));
                self.system_cmd(0x8000, 0x09b0)?;
            },
            Mu1603Mode::Mode1 => {
                self.sensor_program_sequence(0x0083, 0x11dc)?;
                self.sensor_cmd(0x103b, 0x0000)?;
                self.system_cmd(0x2000, 0x0001)?;
                self.system_cmd(0x1200, 0x0003)?;
                std::thread::sleep(Duration::from_millis(10));
                self.system_cmd(0x8000, 0x060c)?;
            },
            Mu1603Mode::Mode2 => {
                self.sensor_program_sequence(0x0083, 0x11dc)?;
                self.sensor_cmd(0x103b, 0x0000)?;
                self.system_cmd(0x2000, 0x0002)?;
                self.system_cmd(0x1200, 0x0004)?;
                std::thread::sleep(Duration::from_millis(10));
                self.system_cmd(0x8000, 0x0666)?;
            },
        }


        // 7. Set exposure and analog gain
        self.set_exposure(0x000a, 0x0cbd)?;
        self.system_cmd(0x0a00, 0x0001)?;
        self.set_exposure(0x000a, 0x0cbd)?;
        self.set_analog_gain(0x610c)?;

        // 7. Start streaming. 
        // After this, frames should be available to read with bulk transfers 
        // on endpoint 0x81.

        self.ven_write(0x01, 0x000f, 0x0003, &[])?;
        std::thread::sleep(Duration::from_millis(10));

        let state = Mu1603State {
            id: 0,
            mode: init_mode,
            analog_gain: AnalogGain::new_from_percent(100),
            exposure: ExposureTime::new_from_us(94_000),
            bitdepth: Mu1603BitDepth::Depth8,
        };
        self.state = Some(state);

        Ok(())
    }

    /// Stop streaming data.
    ///
    /// Presumably this also clears the sensor configuration.
    pub fn stop_stream(&mut self) -> Result<(), Mu1603Error> {
        if !self.is_streaming() { 
            return Ok(()); 
        }

        self.system_cmd(0x0a00, 0x0000)?;
        self.sensor_cmd(0x1000, 0x0000)?;
        self.ven_write(0x01, 0x000f, 0x0000, &[])?;

        let mut wbuf: [u8; 4] = [0; 4];
        self.ven_read(0x17, 0x0000, 0x0000, &mut wbuf)?;
        std::thread::sleep(Duration::from_millis(10));

        self.prev_state = self.state;
        self.state = None;
        Ok(())
    }
}


impl Mu1603 {
    pub fn try_read_frame(&mut self) -> Result<Vec<u8>, Mu1603Error>
    {
        if let Some(state) = self.state { 
            Self::read_frame(&mut self.handle, &state)
        } else { 
            Err(Mu1603Error::NotStreaming)
        }
    }

    fn read_frame(handle: &mut DeviceHandle<Context>, state: &Mu1603State)
        -> Result<Vec<u8>, Mu1603Error>
    {
        const CHUNK: usize = 0x0010_0000;
        let timeout = Duration::from_millis(500);
        let (width, height) = state.mode.dimensions();
        let bpp = state.bitdepth.bpp();
        let frame_len = (width * height) * bpp;
        let mut data = vec![0u8; frame_len];
        let mut chunk = vec![0u8; CHUNK];

        // Issue bulk reads until we've received an entire frame
        let mut cur  = 0;
        let start = std::time::Instant::now();
        let mut loop_total = std::time::Duration::new(0, 0);
        loop {
            let chunk_start = std::time::Instant::now();
            match handle.read_bulk(0x81, &mut chunk, timeout) {
                Ok(rlen) => {
                    let chunk_elapsed = chunk_start.elapsed();
                    loop_total += chunk_elapsed;
                    println!("got {} bytes, took {:?}", rlen, chunk_elapsed);
                    println!("{:?}", chunk[0..0x40].hex_dump());

                    // If the incoming data would overflow the buffer,
                    // just truncate it and copy the remaining bytes
                    let rem = frame_len - cur;
                    let len = if rlen > rem { rem } else { rlen };

                    // Copy into frame buffer
                    data[cur..cur+len].copy_from_slice(&chunk[..len]);
                    cur += len;

                    // If we get less bytes than we requested, this indicates
                    // that the device has finished reading out a frame.
                    if rlen < CHUNK { break; }
                },
                Err(e) => return Err(Mu1603Error::from(e)),
            }
        }
        let elapsed = start.elapsed();
        println!("bulk read total {:?}", loop_total);

        // This really only occurs on the first frame after initialization; 
        // the data is typically truncated, and we can just discard it.
        if cur < frame_len {
            Err(Mu1603Error::FirstFrame)
        } else {
            Ok(data)
        }


        //let mut rem = frame_len;
        //let mut cur = 0;
        //while cur < frame_len {
        //    match handle.read_bulk(0x81, &mut chunk, timeout) {
        //        Ok(recv_len) => {
        //            println!("[*] Got {} bytes", recv_len);
        //            println!("{:?}", chunk[0..0x40].hex_dump());
        //            let copy_len = if recv_len > rem {
        //                println!("  Truncated received length {} to {}", 
        //                    recv_len, rem
        //                );
        //                rem
        //            } else { 
        //                recv_len
        //            };
        //            cur = cur + recv_len;
        //            rem = rem - recv_len; 
        //            let start = cur; 
        //            let end   = cur + recv_len; 
        //            let dst = &mut data[start..end];
        //            dst.copy_from_slice(&chunk[..recv_len]);
        //            if (recv_len < CHUNK) && cur < frame_len {
        //                break;
        //            }
        //            if end > frame_len {
        //                println!("{} would overflow frame {}? ", end, frame_len);
        //                return Err(Mu1603Error::FirstFrame);
        //            }
        //            cur += recv_len;
        //        },
        //        Err(e) => { return Err(Mu1603Error::from(e)); },
        //    }
        //}
        //if cur < frame_len { return Err(Mu1603Error::FirstFrame); }
        //println!();

        //Ok(data)

    }

}


