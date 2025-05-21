
use std::sync::mpsc::{ Sender, Receiver, SendError, TryRecvError };
use glass_mu1603::*;

/// Control messages from the egui thread to the camera thread.
#[derive(Copy, Clone, Debug)]
pub enum ControlMessage {
    ///// Set the exposure time
    //Exposure(usize),

    ///// Set the analog gain
    //AnalogGain(usize),

    /// Update camera settings
    Update(Mu1603Options),

    /// Connect to the camera
    Connect,

    /// Disconnect from the camera
    Disconnect,

    /// Shutdown the camera thread
    Shutdown,
}

#[derive(Copy, Clone, Debug)]
pub enum CameraMessage {
    ThreadInit,

    /// The camera thread has connected to the device
    Connected(Mu1603Options),

    /// The camera thread failed to connect to the device
    ConnectFailure(rusb::Error),

    /// The camera thread has disconnected from the device
    Disconnected,

    /// The camera thread has started streaming frames
    StartStreaming,

    /// The camera thread has acknowledged an update to the camera state
    UpdateAck(Mu1603Options),

    Debug(&'static str),
}

pub struct EguiThreadChannels { 
    /// For receiving a frame from the camera thread
    pub frame_rx: Receiver<usize>,

    /// For receiving updates about the camera thread's state
    pub state_rx: Receiver<CameraMessage>,

    /// For sending requests to the camera thread
    pub ctl_tx: Sender<ControlMessage>,
}
impl EguiThreadChannels {
    //pub fn send_exposure_update(&mut self, x: usize) 
    //    -> Result<(), SendError<ControlMessage>>
    //{
    //    self.ctl_tx.send(ControlMessage::Exposure(x))
    //}

    //pub fn send_again_update(&mut self, x: usize)
    //    -> Result<(), SendError<ControlMessage>>
    //{
    //    self.ctl_tx.send(ControlMessage::AnalogGain(x))
    //}

    pub fn send_connect_request(&mut self)
        -> Result<(), SendError<ControlMessage>>
    {
        self.ctl_tx.send(ControlMessage::Connect)
    }

    pub fn send_disconnect_request(&mut self)
        -> Result<(), SendError<ControlMessage>>
    {
        self.ctl_tx.send(ControlMessage::Disconnect)
    }

    pub fn send_update_request(&mut self, x: Mu1603Options)
        -> Result<(), SendError<ControlMessage>>
    {
        self.ctl_tx.send(ControlMessage::Update(x))
    }

    pub fn send_shutdown_request(&mut self)
        -> Result<(), SendError<ControlMessage>>
    {
        self.ctl_tx.send(ControlMessage::Shutdown)
    }

}



pub struct CameraThreadChannels {
    /// For sending frames to the egui thread
    pub frame_tx: Sender<usize>,

    /// For sending updates about our state to the egui thread
    pub state_tx: Sender<CameraMessage>,

    /// For receiving requests from the egui thread
    pub ctl_rx: Receiver<ControlMessage>,
}
impl CameraThreadChannels {
    pub fn send_state_update(&mut self, msg: CameraMessage) {
        if let Err(send_err) = self.state_tx.send(msg) { 
            println!("Failed to send state update to camera: {:?}, {}", 
                msg, send_err);
        }
    }

    pub fn send_frame_update(&mut self, data: Vec<u8>) {
        self.frame_tx.send(data.len()).unwrap();
    }

}



