use std::sync::{Arc, RwLock};
use std::sync::atomic::AtomicBool;
use glass_common::*;
use chrono;

pub struct AcquisitionState { 
    /// Container for a demosaiced image acquired from the renderer.
    /// The 'glow' backend is expected to write a demosaiced frame here.
    data: Arc<RwLock<PixelData>>,

    /// Signal used to request a demosaiced image from the renderer.
    pending: Arc<AtomicBool>,

    /// Application start time
    session_start: chrono::DateTime<chrono::Local>,

    /// Number of acquired images [during this session]
    count: usize,

    /// Collection of acquired images during this session
    images: Vec<PixelData>,
}
impl AcquisitionState { 
    pub fn new(fmt: PixelFormat, w: usize, h: usize) -> Self { 
        Self { 
            data: Arc::new(RwLock::new(PixelData::new(fmt, w, h))),
            pending: Arc::new(AtomicBool::new(false)),
            count: 0,
            session_start: chrono::Local::now(),
            images: Vec::new(),
        }
    }

    pub fn data(&self) -> Arc<RwLock<PixelData>> {
        self.data.clone()
    }

    pub fn pending(&self) -> Arc<AtomicBool> {
        self.pending.clone()
    }

    pub fn next_filename(&self) -> String { 
        format!("/tmp/{}-{:04}.rgb8.raw", self.session_start.format("%d%m%y-%H%M"), self.count)
    }

}

