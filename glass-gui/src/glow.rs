
use eframe::glow;
use eframe::egui;
use std::sync::{ Mutex, Arc, RwLock, RwLockWriteGuard, RwLockReadGuard };
use eframe::glow::HasContext;

use glass_common::*;
use glass_glow::*;

//use glow_util::*;
//use glow_util::demosaic::DemosaicQuad;

/// Container for all of the state associated with the camera preview.
pub struct PreviewGlow {
    pub preview: Arc<Mutex<Preview>>,
    callback: Arc<eframe::egui_glow::CallbackFn>,
}
impl PreviewGlow {
    pub fn new(rgb_data: Arc<RwLock<PixelData>>) -> Self { 
        let preview = Arc::new(Mutex::new(Preview::new(rgb_data)));
        let p = preview.clone();
        let callback = Arc::new({
            eframe::egui_glow::CallbackFn::new(move |info, painter| 
            {
                p.lock().unwrap().paint(info, painter)
            })
        });

        Self { 
            preview,
            callback,
        }
    }
    pub fn paint(&mut self, rect: egui::Rect) -> egui::PaintCallback {
        egui::PaintCallback {
            rect, callback: self.callback.clone()
        }
    }
    pub fn destroy(&mut self, gl: &glow::Context) {
        self.preview.lock().unwrap().destroy(gl);
    }
}


pub struct Preview {
    program: DemosaicQuad,
    pub raw_data: Arc<RwLock<PixelData>>,
}
impl Preview {
    pub fn new(raw_data: Arc<RwLock<PixelData>>) -> Self { 
        Self { 
            program: DemosaicQuad::new(),
            raw_data,
        }
    }

    pub fn destroy(&mut self, gl: &eframe::glow::Context) {
        self.program.destroy(&gl);
    }

    pub fn paint(&mut self, 
        _info: eframe::egui::PaintCallbackInfo, 
        painter: &eframe::egui_glow::Painter
    ) 
    {
        let gl = painter.gl();
        if !self.program.is_initialized() {
            if let Err(e) = self.program.init(&gl) { 
                panic!("{:?}", e);
            }
        }

        self.program.paint(&gl);
    }
}


