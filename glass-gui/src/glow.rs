
use eframe::glow;
use eframe::egui;
use std::sync::{ Mutex, Arc, RwLock, RwLockWriteGuard, RwLockReadGuard };
use eframe::glow::HasContext;

use glass_common::*;
use glass_glow::*;

/// Container for all of the state associated with the camera preview.
pub struct PreviewGlow {
    pub preview: Arc<Mutex<Preview>>,
    paint_callback: Arc<eframe::egui_glow::CallbackFn>,
    acquire_callback: Arc<eframe::egui_glow::CallbackFn>,
}
impl PreviewGlow {
    pub fn new(rgb_data: Arc<RwLock<PixelData>>, acquire_data: Arc<RwLock<PixelData>>) 
        -> Self
    { 
        let preview = Arc::new(Mutex::new(Preview::new(rgb_data, acquire_data)));

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
    program: DemosaicQuad,
    pub raw_data: Arc<RwLock<PixelData>>,
    pub acquire_data: Arc<RwLock<PixelData>>,
    pub last_frame: PixelData,
}
impl Preview {

    pub fn new(raw_data: Arc<RwLock<PixelData>>, acquire_data: Arc<RwLock<PixelData>>) 
        -> Self
    { 
        let w = raw_data.read().unwrap().width();
        let h = raw_data.read().unwrap().height();
        let format = raw_data.read().unwrap().format();
        Self { 
            program: DemosaicQuad::new(w, h),
            last_frame: PixelData::new(format, w, h),
            acquire_data,
            raw_data,
        }
    }

    pub fn destroy(&mut self, gl: &eframe::glow::Context) {
        self.program.destroy(&gl);
    }

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

        // Upload new data to the texture and paint it. 
        self.program.update_texture(&gl, &self.last_frame.data);
        self.program.paint(&gl);

        if let Ok(mut data) = self.acquire_data.write() {
            unsafe { 

                // NOTE: this is straight up read the framebuffer(including the ui).
                // You probably want to like, render to a texture instead of the fb? 
                //gl.read_pixels(0, 0, 2320, 1740, glow::RGB, glow::UNSIGNED_BYTE, 
                //    glow::PixelPackData::Slice(&mut data.data));

                //gl.active_texture(glow::TEXTURE0);
                //gl.bind_texture(glow::TEXTURE_RECTANGLE, self.program.texture);
                //gl.get_tex_image(glow::TEXTURE_2D, 0, glow::RED, 
                //    glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(&mut data.data)
                //);
            }
            data.increment_frame_id();
            //println!("acquired image?");
        }


    }
}


