#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod input;
mod network;
mod protocol;
mod video;

use std::time::{Duration, Instant};
use crossbeam_channel::Receiver;
use eframe::egui::{self, Color32, ColorImage, Rect, TextureHandle, TextureOptions, Vec2};

use input::InputManager;
use network::NetworkClient;
use video::{VideoCaptureManager, VideoDeviceInfo, VideoFrame};

struct MacConnectApp {
    network: NetworkClient,
    input_mgr: InputManager,
    video_mgr: VideoCaptureManager,
    frame_rx: Receiver<VideoFrame>,

    // UI States
    mac_ip: String,
    mac_port: String,
    devices: Vec<VideoDeviceInfo>,
    selected_device: Option<u32>,
    show_settings: bool,
    video_texture: Option<TextureHandle>,
    last_frame_time: Instant,
    fps_counter: u32,
    current_fps: u32,
    is_fullscreen: bool,
    status_warning: Option<(String, Instant)>,
}

impl MacConnectApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let network = NetworkClient::new().expect("Failed to initialize UDP network client");
        let input_mgr = InputManager::new(network.clone());
        input_mgr.start_hooks();

        let video_mgr = VideoCaptureManager::new();
        let frame_rx = video_mgr.get_frame_receiver();

        let devices = VideoCaptureManager::list_devices();

        // Prefer USB Capture Card devices (UGREEN, Capture, Cam Link, USB Video) over built-in webcams
        let selected_device = devices
            .iter()
            .find(|d| {
                let name_lower = d.name.to_lowercase();
                name_lower.contains("ugreen")
                    || name_lower.contains("capture")
                    || name_lower.contains("cam link")
                    || name_lower.contains("hdmi")
                    || name_lower.contains("usb video")
            })
            .or_else(|| devices.first())
            .map(|d| d.index);

        let app = Self {
            network,
            input_mgr,
            video_mgr,
            frame_rx,
            mac_ip: "192.168.1.".to_string(),
            mac_port: "12345".to_string(),
            devices,
            selected_device,
            show_settings: true,
            video_texture: None,
            last_frame_time: Instant::now(),
            fps_counter: 0,
            current_fps: 0,
            is_fullscreen: false,
            status_warning: None,
        };

        // Auto-start capture for the selected capture card
        if let Some(idx) = app.selected_device {
            app.video_mgr.start_capture(idx);
        }

        app
    }

    fn update_video_texture(&mut self, ctx: &egui::Context) {
        // Drain frames to always get the latest one (zero buffer delay)
        let mut latest_frame = None;
        while let Ok(frame) = self.frame_rx.try_recv() {
            latest_frame = Some(frame);
        }

        if let Some(frame) = latest_frame {
            let color_image = ColorImage::from_rgba_unmultiplied(
                [frame.width as usize, frame.height as usize],
                &frame.data,
            );

            if let Some(ref mut texture) = self.video_texture {
                texture.set(color_image, TextureOptions::LINEAR);
            } else {
                self.video_texture = Some(ctx.load_texture("video-feed", color_image, TextureOptions::LINEAR));
            }

            self.fps_counter += 1;
        }

        if self.last_frame_time.elapsed() >= Duration::from_secs(1) {
            self.current_fps = self.fps_counter;
            self.fps_counter = 0;
            self.last_frame_time = Instant::now();
        }
    }
}

impl eframe::App for MacConnectApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_video_texture(ctx);

        // Always request continuous repainting for smooth 60fps video
        ctx.request_repaint();

        let is_locked = self.input_mgr.is_locked();

        // Safety: Auto-unlock if the application window loses focus
        let is_window_focused = ctx.input(|i| i.raw.focused);
        if !is_window_focused && is_locked {
            self.input_mgr.set_locked(false);
        }

        // Render Top Menu / Settings Bar
        if self.show_settings {
            egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("🖥️ MacConnect");
                    ui.separator();

                    // Connection status
                    let is_conn = self.network.is_connected();
                    let rtt = self.network.get_rtt_ms();
                    if is_conn {
                        ui.colored_label(Color32::from_rgb(0, 220, 100), format!("● Connected ({}ms)", rtt));
                    } else {
                        ui.colored_label(Color32::from_rgb(255, 140, 0), "○ Mac Not Detected");
                    }

                    ui.separator();
                    ui.label("Mac IP:");
                    let ip_resp = ui.add(egui::TextEdit::singleline(&mut self.mac_ip).desired_width(110.0));
                    ui.label("Port:");
                    let port_resp = ui.add(egui::TextEdit::singleline(&mut self.mac_port).desired_width(50.0));

                    if ip_resp.changed() || port_resp.changed() {
                        if let Ok(p) = self.mac_port.parse::<u16>() {
                            self.network.set_target(&self.mac_ip, p);
                        }
                    }

                    ui.separator();

                    // Capture Card Selection
                    ui.label("Capture Device:");
                    let current_name = self
                        .devices
                        .iter()
                        .find(|d| Some(d.index) == self.selected_device)
                        .map(|d| d.name.as_str())
                        .unwrap_or("No Device Found");

                    egui::ComboBox::from_id_source("device_select")
                        .selected_text(current_name)
                        .show_ui(ui, |ui| {
                            for dev in &self.devices {
                                let is_sel = Some(dev.index) == self.selected_device;
                                if ui.selectable_label(is_sel, &dev.name).clicked() {
                                    self.selected_device = Some(dev.index);
                                    self.video_texture = None; // Reset texture for new resolution
                                    self.video_mgr.start_capture(dev.index);
                                }
                            }
                        });

                    if ui.button("🔄 Refresh").clicked() {
                        self.devices = VideoCaptureManager::list_devices();
                    }

                    if self.current_fps > 0 {
                        ui.colored_label(Color32::from_rgb(100, 200, 255), format!("{} FPS", self.current_fps));
                    }

                    ui.separator();

                    // Lock / Unlock Button
                    if is_locked {
                        if ui.button("🔓 UNLOCK (Esc / Ctrl+Alt)").clicked() {
                            self.input_mgr.set_locked(false);
                        }
                    } else {
                        if ui.button("🔒 LOCK / CONTROL MAC").clicked() {
                            if !self.network.is_connected() {
                                self.status_warning = Some((
                                    format!("⚠️ Mac not responding at {}:{}. Ensure MacReceiver is running.", self.mac_ip, self.mac_port),
                                    Instant::now(),
                                ));
                            }
                            self.input_mgr.set_locked(true);
                        }
                    }

                    ui.separator();

                    // Fullscreen Toggle
                    if ui.button(if self.is_fullscreen { "🗗 Windowed" } else { "⛶ Fullscreen" }).clicked() {
                        self.is_fullscreen = !self.is_fullscreen;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.is_fullscreen));
                    }

                    // Toggle menu visibility
                    if ui.button("Hide Bar (F10)").clicked() {
                        self.show_settings = false;
                    }
                });
            });
        }

        // Handle Hotkeys in UI
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Escape) && self.input_mgr.is_locked() {
                self.input_mgr.set_locked(false);
            }
            if i.key_pressed(egui::Key::F10) {
                self.show_settings = !self.show_settings;
            }
            if i.key_pressed(egui::Key::F11) {
                self.is_fullscreen = !self.is_fullscreen;
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.is_fullscreen));
            }
        });

        // Main Video Area
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::BLACK))
            .show(ctx, |ui| {
                let avail_size = ui.available_size();
                let rect = Rect::from_min_size(ui.cursor().min, avail_size);

                if let Some(ref texture) = self.video_texture {
                    ui.painter().image(
                        texture.id(),
                        rect,
                        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                } else {
                    // Placeholder when waiting for capture card
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            if let Some(ref err) = self.video_mgr.get_last_error() {
                                ui.colored_label(Color32::from_rgb(255, 100, 100), format!("❌ Video Capture Error: {}", err));
                                ui.add_space(8.0);
                            } else {
                                ui.heading("📺 Waiting for Mac Mini Video Feed...");
                                ui.add_space(8.0);
                            }
                            ui.label("1. Connect HDMI from Mac Mini to your USB Capture Card.");
                            ui.label("2. Plug USB Capture Card into this PC.");
                            ui.label("3. Select your device from the dropdown above.");
                            ui.add_space(16.0);
                            ui.label("👉 Click anywhere inside this window to lock and control your Mac Mini.");
                            ui.label("⌨️ Press [Escape] or [Ctrl + Alt] anytime to release control back to Windows.");
                        });
                    });
                }

                // Interactive Click to Lock Mouse & Keyboard
                let response = ui.allocate_rect(rect, egui::Sense::click());
                if response.clicked() && !is_locked {
                    if !self.network.is_connected() {
                        self.status_warning = Some((
                            format!("⚠️ Mac not connected at {}:{}. You can still test, but start MacReceiver on your Mac.", self.mac_ip, self.mac_port),
                            Instant::now(),
                        ));
                    }
                    self.input_mgr.set_locked(true);
                }

                // Warning Banner
                if let Some((ref msg, time)) = self.status_warning {
                    if time.elapsed() < Duration::from_secs(4) {
                        let warn_rect = Rect::from_min_size(
                            egui::pos2(rect.center().x - 220.0, rect.top() + 20.0),
                            Vec2::new(440.0, 32.0),
                        );
                        ui.painter().rect_filled(
                            warn_rect,
                            8.0,
                            Color32::from_rgba_unmultiplied(180, 40, 40, 230),
                        );
                        ui.painter().text(
                            warn_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            msg,
                            egui::FontId::proportional(12.0),
                            Color32::WHITE,
                        );
                    }
                }

                // Floating Status Pill Overlay
                let pill_rect = Rect::from_min_size(
                    egui::pos2(rect.left() + 16.0, rect.bottom() - 40.0),
                    Vec2::new(340.0, 28.0),
                );
                ui.painter().rect_filled(
                    pill_rect,
                    14.0,
                    Color32::from_rgba_unmultiplied(20, 20, 20, 200),
                );

                let status_text = if is_locked {
                    "🔒 CONTROLLING MAC (Press ESC or Ctrl+Alt to Exit)"
                } else {
                    "🔓 CLICK WINDOW TO CONTROL MAC"
                };
                let status_color = if is_locked {
                    Color32::from_rgb(0, 255, 120)
                } else {
                    Color32::from_rgb(200, 200, 200)
                };

                ui.painter().text(
                    egui::pos2(pill_rect.left() + 14.0, pill_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    status_text,
                    egui::FontId::proportional(12.0),
                    status_color,
                );
            });
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("MacConnect Client")
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "MacConnect Client",
        native_options,
        Box::new(|cc| Ok(Box::new(MacConnectApp::new(cc)))),
    )
}
