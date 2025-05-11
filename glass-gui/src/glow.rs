
use eframe::glow;
use eframe::egui;
use std::sync::{ Mutex, Arc, RwLock, RwLockWriteGuard, RwLockReadGuard };
use std::sync::atomic::{AtomicBool, Ordering};
use eframe::glow::HasContext;

use glass_common::*;
use glass_glow::*;

/// Container for all of the state associated with the camera preview.
pub struct PreviewGlow {
    /// Container for pixel data and the shader
    pub preview: Arc<Mutex<Preview>>,

    paint_callback: Arc<eframe::egui_glow::CallbackFn>,
    acquire_callback: Arc<eframe::egui_glow::CallbackFn>,
}
impl PreviewGlow {
    pub fn new(
        rgb_data: Arc<RwLock<PixelData>>, 
        acquire_data: Arc<RwLock<PixelData>>,
        acquire_pending: Arc<AtomicBool>,
    ) -> Self
    { 
        let preview = Arc::new(Mutex::new(Preview::new(
                    rgb_data, acquire_data, acquire_pending
        )));

        let p: Arc<Mutex<Preview>> = preview.clone();
        let paint_callback = Arc::new({
            eframe::egui_glow::CallbackFn::new(move |info, painter| 
            {
                p.lock().unwrap().paint(info, painter)
            })
        });

        let p: Arc<Mutex<Preview>> = preview.clone();
        let acquire_callback = Arc::new({
            eframe::egui_glow::CallbackFn::new(move |info, painter| 
            {
                p.lock().unwrap().acquire(info, painter)
            })
        });


        Self { 
            preview,
            paint_callback,
            acquire_callback,
        }
    }

    pub fn get_paint_callback(&mut self, rect: egui::Rect)
        -> egui::PaintCallback 
    {
        egui::PaintCallback { rect, callback: self.paint_callback.clone() }
    }

    pub fn get_acquire_callback(&mut self, rect: egui::Rect)
        -> egui::PaintCallback 
    {
        egui::PaintCallback { rect, callback: self.acquire_callback.clone() }
    }



    pub fn destroy(&mut self, gl: &glow::Context) {
        self.preview.lock().unwrap().destroy(gl);
    }
}


pub struct Preview {
    /// Shader used to demosaic raw data from the sensor
    program: DemosaicQuad,

    /// Most-recent raw data from the sensor
    pub raw_data: Arc<RwLock<PixelData>>,

    pub acquire_data: Arc<RwLock<PixelData>>,
    pub acquire_pending: Arc<AtomicBool>,
    pub last_frame: PixelData,
}
impl Preview {

    pub fn new(
        raw_data: Arc<RwLock<PixelData>>, 
        acquire_data: Arc<RwLock<PixelData>>,
        acquire_pending: Arc<AtomicBool>,
    ) -> Self
    { 
        let w = raw_data.read().unwrap().width();
        let h = raw_data.read().unwrap().height();
        let format = raw_data.read().unwrap().format();
        Self { 
            program: DemosaicQuad::new(w, h),
            last_frame: PixelData::new(format, w, h),
            acquire_data,
            acquire_pending,
            raw_data,
        }
    }

    pub fn destroy(&mut self, gl: &eframe::glow::Context) {
        self.program.destroy(&gl);
    }

    /// Acquire an image
    pub fn acquire(&mut self, 
        _info: eframe::egui::PaintCallbackInfo, 
        painter: &eframe::egui_glow::Painter
    ) 
    {
        let gl = painter.gl();
        if !self.program.is_initialized() {
            return;
        }

        if let Ok(mut data) = self.acquire_data.write() {
            unsafe { 
                //gl.bind_texture(glow::TEXTURE_2D, self.program.texture);

                gl.get_tex_image(glow::TEXTURE_2D, 0, glow::RGB, 
                    glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(&mut data.data)
                );
            }
            data.increment_frame_id();
            println!("acquired image?");
        }

    }

    /// Update the preview window
    pub fn paint(&mut self, 
        _info: eframe::egui::PaintCallbackInfo, 
        painter: &eframe::egui_glow::Painter
    ) 
    {
        let gl = painter.gl();

        // Initialize program
        if !self.program.is_initialized() {
            if let Err(e) = self.program.init(&gl) { 
                panic!("{:?}", e);
            }
        }

        // Get read access to data from the sensor. 
        // If the data has been updated, update our local copy.
        if let Ok(lock) = self.raw_data.read() {
            let remote_id = lock.frame_id();
            if self.last_frame.frame_id() != remote_id {
                self.last_frame.fill_from_slice(&lock.data).unwrap();
            }
        }

        // Upload new data to the texture and actually run the shader
        self.program.update_texture(&gl, &self.last_frame.data);
        self.program.paint(&gl);

        if !self.acquire_pending.load(Ordering::Relaxed) {
            return;
        }
        if let Ok(mut data) = self.acquire_data.write() {
            unsafe { 

                // NOTE: This is straight up reading the framebuffer (including the ui!).
                // You probably want to like, create a different framebuffer object and
                // render into it, then read from it like this?
                //gl.read_pixels(0, 0, 2320, 1740, glow::RGB, glow::UNSIGNED_BYTE, 
                //    glow::PixelPackData::Slice(&mut data.data));

                //gl.active_texture(glow::TEXTURE0);
                //gl.bind_texture(glow::TEXTURE_RECTANGLE, self.program.texture);
                //gl.get_tex_image(glow::TEXTURE_2D, 0, glow::RED, 
                //    glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(&mut data.data)
                //);

                // NOTE: It seems like this returns RGB8, but only the blue pixels have values??
                // This might be reading the original texture (not the newly-rendered image)
                gl.get_tex_image(glow::TEXTURE_2D, 0, glow::RGB, 
                    glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(&mut data.data)
                );

            }
            data.increment_frame_id();
            //println!("acquired image?");
        }


    }
}


