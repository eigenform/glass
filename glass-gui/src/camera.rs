
use rusb::{ Context, UsbContext, Device, DeviceHandle, DeviceDescriptor };
use std::sync::mpsc::{ Sender, Receiver, SendError, TryRecvError };
use std::sync::{ Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard };

use std::time::Duration;
use crate::ipc::*;
use glass_mu1603::*;
use rand::prelude::*;
use glass_common::*;
use bayer;

pub fn spawn_camera_thread(chan: CameraThreadChannels, rgb_data: Arc<RwLock<PixelData>>)
    -> std::thread::JoinHandle<Result<(), CameraThreadError>>
{
    let handle = std::thread::spawn(move || { 
        let mut state = CameraThreadState::new(chan, rgb_data);
        state.main_loop()
    });
    handle
}


#[derive(Debug)]
pub enum CameraThreadError {
    Terminated,
}

pub struct CameraThreadState {
    ctx: Context,
    chan: CameraThreadChannels,
    dummy: bool,
    streaming: bool,
    cam: Option<Mu1603>,

    rgb_data: Arc<RwLock<PixelData>>,

}
impl CameraThreadState {
    pub fn new(chan: CameraThreadChannels, rgb_data: Arc<RwLock<PixelData>>) -> Self { 
        Self { 
            ctx: Context::new().unwrap(),
            chan,
            cam: None,
            dummy: false,
            streaming: false,
            rgb_data,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.cam.is_some()
    }

    pub fn is_streaming(&self) -> bool {
        if let Some(cam) = &self.cam {
            cam.is_streaming()
        } else {
            false
        }
    }

    pub fn is_device_present(&self) -> rusb::Result<bool> {
        let devices = self.ctx.devices()?;
        for device in devices.iter() {
            let desc = device.device_descriptor()?;
            if desc.vendor_id() == 0x0547 && desc.product_id() == 0x3016 {
                return Ok(true);
            }
        }
        Ok(false)
    }
}


impl CameraThreadState {
    pub fn main_loop(&mut self) -> Result<(), CameraThreadError> 
    {
        let mut iter: usize = 0;
        self.chan.send_state_update(CameraMessage::ThreadInit);
        'top: loop 
        { 
            iter += 1;
            match self.chan.ctl_rx.try_recv() {
                // No message, nothing for us to do
                Err(TryRecvError::Empty) => {
                    std::thread::sleep(Duration::from_millis(1));
                },
                // Handle a message from the egui thread
                Ok(msg) => {
                    let res = self.handle_msg(msg);
                    match res {
                        Ok(()) => {
                            std::thread::sleep(Duration::from_millis(1));
                        },
                        Err(e) => return Err(e),
                    }
                },
                // Channel disconnected, guess I'll die ¯\_(ツ)_/¯
                Err(TryRecvError::Disconnected) => {
                    println!("disconnected");
                    break 'top;
                },
            }

            //self.simulate_frame_write(iter);

            if let Some(cam) = &mut self.cam {
                match cam.try_read_frame() {
                    Ok(data) => {
                        //self.chan.send_frame_update(data);
                        if let Ok(mut lock) = self.rgb_data.try_write() {
                            if let Err(e) = lock.fill_from_slice(&data) { 
                                println!("{}", e);
                            } else { 
                                lock.increment_frame_id();
                            }
                        } else { 
                            println!("try_write failed");

                        }
                    },
                    Err(e) => { 
                        match e {
                            Mu1603Error::NotStreaming => {
                                std::thread::sleep(Duration::from_millis(1));
                            },
                            Mu1603Error::Rusb(re) => {
                            },
                            Mu1603Error::FirstFrame => {
                            },
                            Mu1603Error::Unimplemented => {
                                unreachable!();
                            },
                            Mu1603Error::FailedSensorCmd(_, _) => {
                                unreachable!();
                            },
                        }
                    },
                }
            }
        }
        Ok(())
    }
}

impl CameraThreadState {
    pub fn handle_connect(&mut self) -> Result<(), CameraThreadError> {
        // Ignore this message if we're already connected. 
        if self.is_connected() {
            return Ok(());
        }

        // Try to connect to the camera
        let resp = match Mu1603::try_open(&mut self.ctx) { 
            Ok(mut cam) => {
                cam.start_stream(Mu1603Mode::Mode1).unwrap();
                self.cam = Some(cam);
                CameraMessage::Connected
            },
            Err(e) => CameraMessage::ConnectFailure(e),
        };

        self.chan.send_state_update(resp);
        Ok(())
    }
    pub fn handle_disconnect(&mut self) -> Result<(), CameraThreadError> 
    {
        if let Some(ref mut cam) = &mut self.cam {
            cam.stop_stream().unwrap();
        }
        Ok(())
    }

    pub fn handle_update(&mut self, state: Mu1603State) 
        -> Result<(), CameraThreadError> 
    {
        println!("got upd msg {:?}", state);
        Ok(())
    }

    pub fn handle_msg(&mut self, msg: ControlMessage) 
        -> Result<(), CameraThreadError>
    {
        println!("got {:?}", msg);
        match msg { 
            ControlMessage::Connect => {
                self.handle_connect()
            },
            ControlMessage::Update(state) => {
                self.handle_update(state)
            },
            ControlMessage::Disconnect => {
                self.handle_disconnect()
            },
            ControlMessage::Shutdown => {
                Err(CameraThreadError::Terminated)
            },
        }
    }

}


