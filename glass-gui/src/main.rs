#![allow(unused_imports)]
#![allow(dead_code)]
#![feature(portable_simd)]

mod log;
mod camera;
mod ipc;
mod glow;
mod app;
mod acquire; 

use std::sync::{Arc, RwLock};
use glass_common::*;
use glass_mu1603::Mu1603Mode;

fn main() -> Result<(), eframe::Error> {

    // NOTE: This default size for the viewport is for my big 4K displays.

    //env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([3000.0, 2160.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    // Create channels shared between the threads
    let (frame_tx, frame_rx) = std::sync::mpsc::channel();
    let (ctl_tx, ctl_rx)     = std::sync::mpsc::channel();
    let (state_tx, state_rx) = std::sync::mpsc::channel();
    let egui_chan = ipc::EguiThreadChannels {
        frame_rx, ctl_tx, state_rx
    };
    let camera_chan = ipc::CameraThreadChannels {
        frame_tx, ctl_rx, state_tx
    };

    // FIXME: This needs to scale with the configured camera resolution. 
    // We are assuming use of mode 1. 
    let rgb_data = Arc::new(RwLock::new(
        PixelData::new(
            PixelFormat::Bayer8(BayerPattern::BGGR), 
            Mu1603Mode::Mode1.width(), 
            Mu1603Mode::Mode1.height()
        )
    ));
    let rgb_data_clone = rgb_data.clone();

    // Spawn the camera thread. 
    // NOTE: We expect the egui thread to terminate the 
    // camera thread before it returns. 
    let camera_thread = camera::spawn_camera_thread(camera_chan, rgb_data);

    // Block until the egui thread has finished
    let egui_thread   = eframe::run_native(
        "toup-acquire",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Box::new(app::MyApp::new(cc, egui_chan, rgb_data_clone))
        }),
    );

    println!("Egui thread exited with {:?}", egui_thread);
    println!("Waiting for camera thread shutdown ...");
    let camera_thread_res = camera_thread.join();
    println!("Camera thread exited with {:?}", camera_thread_res);
    egui_thread
}

