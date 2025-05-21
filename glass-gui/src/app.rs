
use eframe::{
    self, egui, egui_glow
};

use std::sync::{ Arc, RwLock, Mutex };
use std::sync::mpsc::{ Sender, Receiver, SendError, TryRecvError };
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::*;

use crate::log::*;
use crate::glow::*;
use crate::ipc::*;
use crate::acquire::*;
use glass_mu1603::*;
use glass_common::*;

#[derive(Debug)]
pub enum AppError {
    IpcFailure(ControlMessage),
}

/// Ephemeral state of the UI elements. 
#[derive(Debug)]
struct RequestedSettings { 
    pub mode: Mu1603Mode,
    pub exposure_ms: usize,
    pub analog_gain_percent: usize,
}
impl Default for RequestedSettings { 
    fn default() -> Self { 
        Self { 
            exposure_ms: 94,
            analog_gain_percent: 100,
            mode: Mu1603Mode::Mode1
        }
    }
}

pub struct MyApp {
    // Channels used to communicate with the camera thread
    chan: EguiThreadChannels,

    // Queue of log entries to display in the UI
    log_entries: VecDeque<LogEntry>,

    /// The current state of the camera.
    cam_options: Option<Mu1603Options>,

    /// Reflecting the state of requested camera settings [shown in the UI]
    req_settings: RequestedSettings,

    /// State associated with the preview window
    preview_glow: PreviewGlow,

    /// State associated with image acquisition
    acquire: AcquisitionState,

    // FIXME: Replace use of these with 'acquire'
    /// Container for a demosaiced image acquired from the renderer
    acquire_data: Arc<RwLock<PixelData>>,
    /// Used to acquire an image from the renderer
    acquire_pending: Arc<AtomicBool>,

}
impl MyApp {
    pub fn new(cc: &eframe::CreationContext<'_>, chan: EguiThreadChannels,
        rgb_data: Arc<RwLock<PixelData>>,
    ) -> Self 
    { 
        // Adjust text size so I don't have to scale up the DPI
        let ctx = &cc.egui_ctx;
        use egui::FontFamily::{ Monospace, Proportional };
        let mut style = (*ctx.style()).clone();
        style.text_styles = [
            (egui::TextStyle::Heading, egui::FontId::new(30.0, Proportional)),
            (egui::TextStyle::Body, egui::FontId::new(20.0, Proportional)),
            (egui::TextStyle::Monospace, egui::FontId::new(20.0, Monospace)),
            (egui::TextStyle::Button, egui::FontId::new(20.0, Proportional)),
            (egui::TextStyle::Small, egui::FontId::new(16.0, Proportional)),
        ].into();
        ctx.set_style(style);

        //// State for glow usage via paint callbacks
        //let gl = cc.gl.as_ref().expect("No glow backend?");

        // FIXME: This needs to match the dimensions of 'rgb_data'
        // FIXME: Replace these with [AcquisitionState]
        let acquire_data = Arc::new(RwLock::new(PixelData::new(
            PixelFormat::RGB8, 
            Mu1603Mode::Mode1.width(), 
            Mu1603Mode::Mode1.height()
        )));
        let acquire_data_clone = acquire_data.clone();
        let acquire_pending = Arc::new(AtomicBool::new(false));

        Self {
            chan,
            req_settings: RequestedSettings::default(),
            log_entries: VecDeque::new(),
            cam_options: None,
            preview_glow: PreviewGlow::new(rgb_data, acquire_data_clone, acquire_pending.clone()),
            acquire: AcquisitionState::new(
                PixelFormat::RGB8, 
                Mu1603Mode::Mode1.width(),
                Mu1603Mode::Mode1.height(),
            ),
            // FIXME: Replace these with [AcquisitionState]
            acquire_data,
            acquire_pending,
        }
    }

    pub fn camera_connected(&self) -> bool {
        self.cam_options.is_some()
    }

    pub fn push_log(&mut self, evt: LogEvent) {
        println!("log: {:?}", evt);
        self.log_entries.push_back(LogEntry::new(evt))
    }
}


/// Interactions with other threads
impl MyApp {

    // FIXME: This is fine for *testing* acquisition, for now.
    //
    // FIXME: What if we never acquire the read lock? :x
    //
    // FIXME: Maybe you should do this in a different thread?
    //        Right now, we're just doing it in the egui thread.
    //
    pub fn check_acquisition_thread(&mut self) {
        if self.acquire_pending.load(Ordering::Relaxed) { 
            use std::io::Write;
            if let Ok(acquire_data) = self.acquire_data.read() {
                let filename = self.acquire.next_filename();
                let mut f = std::fs::File::create(&filename).unwrap();
                f.write_all(&acquire_data.data).unwrap();
                self.acquire_pending.store(false, Ordering::Relaxed);
                println!("wrote {}", filename);
            }
        }
    }

    // FIXME: Both of these only consume at most *one* message.
    //        Are there any cases where we might want to handle many at once?
    pub fn check_camera_thread(&mut self) {

        // Receive frames from the camera thread.
        match self.chan.frame_rx.try_recv() {
            Ok(msg) => {
                // NOTE: This is just simulating some work for now
                self.log_entries.push_back(LogEntry::new(LogEvent::Msg(msg)));
            },
            Err(TryRecvError::Empty) => {},
            Err(TryRecvError::Disconnected) => {
                self.log_entries.push_back(
                    LogEntry::new(LogEvent::LostThread)
                );
            },
        }

        // Receive updates about the state of the camera thread.
        match self.chan.state_rx.try_recv() {
            Ok(msg) => {
                self.push_log(LogEvent::CameraMsg(msg));
                match msg { 
                    CameraMessage::Connected(state) => {
                        self.cam_options = Some(state);
                    },
                    CameraMessage::Disconnected => {
                        self.cam_options = None;
                    },
                    CameraMessage::ThreadInit => {},
                    CameraMessage::StartStreaming => {},
                    CameraMessage::UpdateAck(state) => {},
                    CameraMessage::ConnectFailure(e) => {
                        println!("connect failure: {:?}", e);
                        self.cam_options = None;
                    },
                    CameraMessage::Debug(msg) => {
                        println!("{}", msg);
                    }
                }
            },
            Err(TryRecvError::Empty) => {},
            Err(TryRecvError::Disconnected) => {
                self.cam_options = None;
                self.log_entries.push_back(
                    LogEntry::new(LogEvent::LostThread)
                );
            },
        }
    }
}

/// For actually drawing the user interface
impl MyApp {

    pub fn draw_camera_control(&mut self, ui: &mut egui::Ui) {
        let camera_connected = self.camera_connected();

        // Camera connection state
        ui.heading("Camera");
        ui.vertical_centered(|ui| { 
            ui.spacing_mut().item_spacing.y = 10.0;
            let connect_button_text = if camera_connected {
                "Disconnect"
            } else { 
                "Connect"
            };
            let connect_label = if camera_connected {
                egui::RichText::new("Connected")
                    .color(egui::Color32::LIGHT_GREEN)
            } else {
                egui::RichText::new("Not Connected")
                    .color(egui::Color32::RED)
            };

            let connect_button = egui::Button::new(connect_button_text)
                .min_size([100.0,50.0].into());
            let connect_button_resp = ui.add(connect_button);
            ui.label(connect_label);

            if connect_button_resp.clicked() {
                if !camera_connected {
                    self.chan.send_connect_request().unwrap();
                } else {
                    self.chan.send_disconnect_request().unwrap();
                }
            }
        });
        ui.separator();
    }

    pub fn draw_settings_control(&mut self, ui: &mut egui::Ui)
    {
        let camera_connected = self.camera_connected();
        let (gain_desync, exp_desync, mode_desync) = if let Some(state) = self.cam_options {
            (state.analog_gain_percent() != self.req_settings.analog_gain_percent,
             state.exposure_ms() != self.req_settings.exposure_ms,
             state.mode != self.req_settings.mode)
        } 
        else { 
            (true, true, true)
        };
        let desync = gain_desync || exp_desync || mode_desync;
        let sync_label = if desync {
            egui::RichText::new("Synchronized")
                .color(egui::Color32::LIGHT_GREEN)
        } else {
            egui::RichText::new("Not Synchronized")
                .color(egui::Color32::RED)
        };


        ui.heading("Settings");
        ui.vertical_centered(|ui| { 
            ui.spacing_mut().slider_width = ui.available_width() * 0.45;
            ui.spacing_mut().item_spacing.y = 10.0;

            let mode_desc = self.req_settings.mode.description();
            let mode_mut = &mut self.req_settings.mode;
            let res_select = egui::ComboBox::from_label("Resolution")
                .selected_text(format!("{}", mode_desc));
            res_select.show_ui(ui, |ui| {
                ui.selectable_value(mode_mut, Mu1603Mode::Mode0, 
                    Mu1603Mode::Mode0.description()
                );
                ui.selectable_value(mode_mut, Mu1603Mode::Mode1, 
                    Mu1603Mode::Mode1.description()
                );
                ui.selectable_value(mode_mut, Mu1603Mode::Mode2, 
                    Mu1603Mode::Mode2.description()
                );
            });
            ui.add_space(20.0);

            let exp_range  = 32..=256;
            let exp_mut = &mut self.req_settings.exposure_ms;
            let exp_slider = egui::Slider::new(exp_mut, exp_range)
                .suffix("ms")
                .text("Exposure")
                .drag_value_speed(0.25)
                .trailing_fill(true)
                .custom_formatter(|val, _| format!("{:3}", val));

            let gain_range = 100..=300;
            let gain_mut = &mut self.req_settings.analog_gain_percent;
            let again_slider = egui::Slider::new(gain_mut, gain_range)
                .suffix("%")
                .text("Analog Gain")
                .drag_value_speed(0.25)
                .trailing_fill(true)
                .custom_formatter(|val, _| format!("{:3}", val));

            ui.add(exp_slider);
            ui.add(again_slider);
            ui.add_space(20.0);

            let apply_button = egui::Button::new("Apply")
                .min_size([100.0,50.0].into());
            let apply_button_resp = ui.add_enabled(camera_connected, apply_button);
            if apply_button_resp.enabled() && apply_button_resp.clicked() {
                println!("{:?}", self.req_settings);
                apply_button_resp.highlight();
            }
            if camera_connected {
                ui.label(sync_label);
            }
        });


        ui.separator();
    }

    pub fn draw_acquisition_control(&mut self, ui: &mut egui::Ui) {
        use chrono;

        let camera_connected = self.camera_connected();
        ui.heading("Acquisition");
        ui.vertical_centered(|ui| { 
            let snap_button = egui::Button::new("Acquire")
                .min_size([100.0,50.0].into());

            let snap_button_resp = ui.add_enabled(camera_connected, snap_button);

            if snap_button_resp.enabled() && snap_button_resp.clicked() {
                snap_button_resp.highlight();
                self.push_log(LogEvent::Acquire);
                self.acquire_pending.store(true, Ordering::Relaxed);
            }
        });
        ui.separator();
    }

    pub fn draw_log_display(&mut self, ui: &mut egui::Ui) {
        let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
        let scroll_area = egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_width(f32::INFINITY)
            .stick_to_bottom(true);

        ui.heading("Debug Log");
        ui.add_space(10.0);
        scroll_area.show_rows(ui, row_height, self.log_entries.len(),
            |ui, row_range| 
        {
            for row in self.log_entries.range(row_range) {
                ui.monospace(format!("{}", row));
            }
        });
        ui.add_space(10.0);
    }

    pub fn draw_preview(&mut self, ui: &mut egui::Ui) {
        let (rect, _) = ui.allocate_exact_size(
            egui::Vec2::new(
                Mu1603Mode::Mode1.width() as f32, 
                Mu1603Mode::Mode1.height() as f32
            ), 
            egui::Sense::hover()
        );
        // Use the 'glow' renderer to actually draw the contents
        ui.painter().add(self.preview_glow.get_paint_callback(rect));
    }

    pub fn draw_ui(&mut self, ctx: &egui::Context) {
        // Draw the control panel on the left side panel
        egui::SidePanel::left("Control Panel").show(ctx, |panel| 
        {
            panel.spacing_mut().item_spacing.y = 25.0;
            panel.set_width(384.0);
            panel.expand_to_include_x(384.0);

            self.draw_camera_control(panel);
            self.draw_settings_control(panel);
            self.draw_acquisition_control(panel);

            panel.heading("Info");
            panel.monospace(format!("egui frame: {:010}", ctx.frame_nr()));

        });
        // Draw the log on the bottom panel
        egui::TopBottomPanel::bottom("Log").show(ctx, |log| 
        {
            log.set_height(200.0);
            self.draw_log_display(log);
        });

        // Draw the current preview frame in the central panel
        egui::CentralPanel::default().show(ctx, |viewport| 
        {
            viewport.vertical_centered(|ui| {
                self.draw_preview(ui);
            });
        });
    }
}


impl eframe::App for MyApp {
    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        if let Some(gl) = gl {
            self.preview_glow.destroy(gl);
        }
        self.chan.ctl_tx.send(ControlMessage::Shutdown).unwrap();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        // Explicitly repaint on each frame. 
        // We expect to be ingesting preview updates constantly. 
        ctx.request_repaint();

        // Handle pending messages from the camera thread
        self.check_camera_thread();

        // Handle pending messages from the acquisition thread
        self.check_acquisition_thread();

        // Draw the UI
        self.draw_ui(ctx);
    }
}

