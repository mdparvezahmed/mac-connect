#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod input;
mod network;
mod protocol;
mod video;
mod vnc;

use std::time::{Duration, Instant};
use crossbeam_channel::Receiver;
use eframe::egui::{self, Color32, ColorImage, Rect, TextureHandle, TextureOptions};

use input::InputManager;
use network::NetworkClient;
use video::{VideoCaptureManager, VideoDeviceInfo, VideoFrame};
use vnc::{keysym, VncManager, VncState};

/// UDP port the MacReceiver listens on by default.
const DEFAULT_CAPTURE_PORT: u16 = 12345;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ClientMode {
    Vnc,
    CaptureCard,
}

struct MacConnectApp {
    mode: ClientMode,

    // VNC Engine (Default)
    vnc_mgr: VncManager,
    vnc_frame_rx: Receiver<VideoFrame>,
    vnc_ip: String,
    vnc_port: String,
    vnc_password: String,
    vnc_is_locked: bool,
    last_vnc_pointer: (u16, u16, u8),

    // Capture Card Engine (Default / Hardware)
    network: NetworkClient,
    input_mgr: InputManager,
    video_mgr: VideoCaptureManager,
    capture_frame_rx: Receiver<VideoFrame>,
    devices: Vec<VideoDeviceInfo>,
    selected_device: Option<u32>,
    capture_ip: String,
    capture_port: String,

    // UI States
    show_settings: bool,
    video_texture: Option<TextureHandle>,
    last_frame_time: Instant,
    fps_counter: u32,
    current_fps: u32,
    is_fullscreen: bool,
    last_vnc_dim: (u32, u32),
    cursor_hidden_by_us: bool,
    link_error: Option<String>,
}

impl MacConnectApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Initialize VNC Engine
        let vnc_mgr = VncManager::new();
        let vnc_frame_rx = vnc_mgr.get_frame_receiver();

        // Initialize Capture Card Engine (Default)
        // Auto-discovery starts immediately: the client broadcasts on the LAN
        // and links itself to whichever MacReceiver answers, so the common case
        // needs no IP typed in at all.
        let network = NetworkClient::new(DEFAULT_CAPTURE_PORT)
            .expect("Failed to initialize UDP network client");
        let input_mgr = InputManager::new(network.clone());
        input_mgr.start_hooks();

        let mut video_mgr = VideoCaptureManager::new();
        let capture_frame_rx = video_mgr.get_frame_receiver();
        let devices = VideoCaptureManager::list_devices();

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

        if let Some(idx) = selected_device {
            let _ = video_mgr.start_capture(idx);
        }

        Self {
            mode: ClientMode::CaptureCard, // Default mode: HDMI Capture Card (0ms latency, 60fps)

            vnc_mgr,
            vnc_frame_rx,
            vnc_ip: "192.168.1.".to_string(),
            vnc_port: "5900".to_string(),
            vnc_password: "".to_string(),
            vnc_is_locked: false,
            last_vnc_pointer: (0, 0, 0),

            network,
            input_mgr,
            video_mgr,
            capture_frame_rx,
            devices,
            selected_device,
            capture_ip: String::new(),
            capture_port: DEFAULT_CAPTURE_PORT.to_string(),

            show_settings: true,
            video_texture: None,
            last_frame_time: Instant::now(),
            fps_counter: 0,
            current_fps: 0,
            is_fullscreen: false,
            last_vnc_dim: (0, 0),
            cursor_hidden_by_us: false,
            link_error: None,
        }
    }

    fn update_video_texture(&mut self, ctx: &egui::Context) {
        let frame_rx = match self.mode {
            ClientMode::Vnc => &self.vnc_frame_rx,
            ClientMode::CaptureCard => &self.capture_frame_rx,
        };

        let mut latest_frame = None;
        while let Ok(frame) = frame_rx.try_recv() {
            latest_frame = Some(frame);
        }

        if let Some(frame) = latest_frame {
            self.last_vnc_dim = (frame.width, frame.height);
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
            ctx.request_repaint();
        }

        if self.last_frame_time.elapsed() >= Duration::from_secs(1) {
            self.current_fps = self.fps_counter;
            self.fps_counter = 0;
            self.last_frame_time = Instant::now();
        }
    }

    /// Hides the Windows cursor while the Mac owns the pointer, and brings it
    /// straight back the moment control is released.
    ///
    /// We talk to Win32 directly instead of using `egui::CursorIcon::None`: the
    /// low-level hooks swallow the very mouse events winit relies on to keep its
    /// cached cursor state in sync, so winit would skip the "show it again" call
    /// and leave the pointer invisible over our window until it wandered out.
    /// Called once per frame from the UI thread, which is the thread that owns
    /// the cursor display counter.
    fn sync_cursor_visibility(&mut self, hide: bool) {
        if hide {
            // Re-assert on every frame something else turned the cursor back on.
            if !self.cursor_hidden_by_us || input::os_cursor_is_showing() {
                input::force_os_cursor_visible(false);
                self.cursor_hidden_by_us = true;
            }
        } else if self.cursor_hidden_by_us {
            input::force_os_cursor_visible(true);
            self.cursor_hidden_by_us = false;
        }
    }
}

impl eframe::App for MacConnectApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_video_texture(ctx);
        ctx.request_repaint_after(Duration::from_millis(16));

        // Safety: Auto-unlock if the application window loses focus
        let is_window_focused = ctx.input(|i| i.raw.focused);
        if !is_window_focused {
            if self.input_mgr.is_locked() {
                self.input_mgr.set_locked(false);
            }
            self.vnc_is_locked = false;
        }

        // Read the lock state *after* the focus check so the cursor below never
        // lags a frame behind an auto-unlock.
        let is_capture_locked = self.input_mgr.is_locked();
        self.sync_cursor_visibility(is_capture_locked);

        // Show whichever Mac auto-discovery latched onto, without fighting the
        // user for the text box while they are typing an address themselves.
        if ctx.memory(|m| m.focused()).is_none() {
            if let Some(ip) = self.network.target_ip() {
                if self.capture_ip != ip {
                    self.capture_ip = ip;
                }
            }
        }

        // Top Navigation & Configuration Bar
        if self.show_settings {
            egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("🖥️ MacConnect");
                    ui.separator();

                    // Mode Toggle
                    ui.selectable_value(&mut self.mode, ClientMode::Vnc, "🌐 VNC");
                    ui.selectable_value(&mut self.mode, ClientMode::CaptureCard, "🔌 Capture Card");

                    ui.separator();

                    match self.mode {
                        ClientMode::Vnc => {
                            ui.label("Mac IP:");
                            let ip_edit = ui.add(egui::TextEdit::singleline(&mut self.vnc_ip).desired_width(110.0));

                            ui.label("Port:");
                            let port_edit = ui.add(egui::TextEdit::singleline(&mut self.vnc_port).desired_width(45.0));

                            ui.label("Password:");
                            let pwd_edit = ui.add(egui::TextEdit::singleline(&mut self.vnc_password).password(true).desired_width(85.0));

                            let enter_pressed = (ip_edit.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)))
                                || (port_edit.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)))
                                || (pwd_edit.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)));

                            if self.vnc_mgr.is_connected() {
                                if ui.button("🔴 Disconnect").clicked() {
                                    self.vnc_mgr.disconnect();
                                    self.vnc_is_locked = false;
                                    self.video_texture = None;
                                }

                                ui.separator();
                                if self.vnc_is_locked {
                                    if ui.button("🔓 UNLOCK (Esc / Ctrl+Alt)").clicked() {
                                        self.vnc_is_locked = false;
                                    }
                                } else {
                                    if ui.button("🔒 LOCK MAC").clicked() {
                                        self.vnc_is_locked = true;
                                    }
                                }
                            } else {
                                if ui.button("🟢 Connect").clicked() || enter_pressed {
                                    if let Ok(p) = self.vnc_port.parse::<u16>() {
                                        self.video_texture = None;
                                        self.vnc_mgr.connect(self.vnc_ip.clone(), p, self.vnc_password.clone());
                                    }
                                }
                            }

                            ui.separator();

                            // VNC Status Badge (compact to prevent pushing buttons off-screen)
                            let vnc_state = self.vnc_mgr.get_state();
                            match vnc_state {
                                VncState::Connected { width, height, .. } => {
                                    if self.vnc_is_locked {
                                        ui.colored_label(Color32::from_rgb(0, 255, 120), format!("🔒 Locked ({}x{})", width, height));
                                    } else {
                                        ui.colored_label(Color32::from_rgb(0, 220, 100), format!("● Connected ({}x{})", width, height));
                                    }
                                }
                                VncState::Connecting => {
                                    ui.colored_label(Color32::from_rgb(255, 200, 0), "⏳ Connecting...");
                                }
                                VncState::Disconnected => {
                                    ui.colored_label(Color32::from_rgb(180, 180, 180), "○ Disconnected");
                                }
                                VncState::Error(ref err) => {
                                    let short_msg = if err.len() > 28 {
                                        format!("❌ {}...", &err[..25])
                                    } else {
                                        format!("❌ {}", err)
                                    };
                                    ui.colored_label(Color32::from_rgb(255, 80, 80), short_msg)
                                        .on_hover_text(format!("Error details:\n{}", err));
                                }
                            }
                        }

                        ClientMode::CaptureCard => {
                            ui.label("Device:");
                            let current_name = self
                                .devices
                                .iter()
                                .find(|d| Some(d.index) == self.selected_device)
                                .map(|d| d.name.as_str())
                                .unwrap_or("No Device Found");

                            egui::ComboBox::from_id_source("device_select")
                                .selected_text(current_name)
                                .width(120.0)
                                .show_ui(ui, |ui| {
                                    for dev in &self.devices {
                                        let is_sel = Some(dev.index) == self.selected_device;
                                        if ui.selectable_label(is_sel, &dev.name).clicked() {
                                            self.selected_device = Some(dev.index);
                                            self.video_texture = None;
                                            self.video_mgr.start_capture(dev.index);
                                        }
                                    }
                                });

                            if ui.button("🔄").clicked() {
                                self.devices = VideoCaptureManager::list_devices();
                            }

                            ui.separator();
                            ui.label("Mac IP:");
                            let ip_edit = ui.add(egui::TextEdit::singleline(&mut self.capture_ip).desired_width(105.0));

                            ui.label("Port:");
                            let port_edit = ui.add(egui::TextEdit::singleline(&mut self.capture_port).desired_width(45.0));

                            let enter_pressed = (ip_edit.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)))
                                || (port_edit.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)));

                            if self.network.is_connected() {
                                if ui.button("🔴 Unlink").clicked() {
                                    self.network.disconnect();
                                    self.input_mgr.set_locked(false);
                                }

                                ui.separator();
                                if is_capture_locked {
                                    if ui.button("🔓 UNLOCK (Esc / Ctrl+Alt)").clicked() {
                                        self.input_mgr.set_locked(false);
                                    }
                                } else {
                                    if ui.button("🔒 LOCK MAC").clicked() {
                                        self.input_mgr.set_locked(true);
                                    }
                                }
                            } else {
                                if ui.button("🟢 Link").clicked() || enter_pressed {
                                    let port = self.capture_port.parse::<u16>().ok();
                                    let ok = match port {
                                        Some(p) => self.network.set_target(&self.capture_ip, p),
                                        None => false,
                                    };
                                    self.link_error = if ok {
                                        None
                                    } else {
                                        Some(format!(
                                            "\"{}:{}\" is not a valid address",
                                            self.capture_ip.trim(),
                                            self.capture_port.trim()
                                        ))
                                    };
                                }

                                if self.network.is_discovering() {
                                    ui.label("🔍 Searching LAN...");
                                } else if ui.button("🔍 Auto").clicked() {
                                    self.network.disconnect();
                                    let port = self.capture_port
                                        .parse::<u16>()
                                        .unwrap_or(DEFAULT_CAPTURE_PORT);
                                    self.network.set_auto_discover(true, port);
                                    self.link_error = None;
                                }

                                if let Some(ref err) = self.link_error {
                                    ui.colored_label(Color32::from_rgb(255, 90, 90), err);
                                }
                            }

                            ui.separator();
                            let is_conn = self.network.is_connected();
                            let rtt = self.network.get_rtt_ms();
                            if is_conn {
                                if is_capture_locked {
                                    ui.colored_label(Color32::from_rgb(0, 255, 120), format!("🔒 Locked ({}ms)", rtt));
                                } else {
                                    ui.colored_label(Color32::from_rgb(0, 220, 100), format!("● Linked ({}ms)", rtt));
                                }
                            } else {
                                ui.colored_label(Color32::from_rgb(255, 140, 0), "○ UDP Unlinked");
                            }
                        }
                    }

                    if self.current_fps > 0 {
                        ui.separator();
                        ui.colored_label(Color32::from_rgb(100, 200, 255), format!("{} FPS", self.current_fps));
                    }

                    ui.separator();

                    // Fullscreen Toggle
                    if ui.button(if self.is_fullscreen { "🗗 Windowed" } else { "⛶ Fullscreen" }).clicked() {
                        self.is_fullscreen = !self.is_fullscreen;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.is_fullscreen));
                    }

                    if ui.button("Hide Bar (F10)").clicked() {
                        self.show_settings = false;
                    }
                });
            });
        }

        // Handle Global Shortcuts
        ctx.input(|i| {
            if i.key_pressed(egui::Key::F10) {
                self.show_settings = !self.show_settings;
            }
            if i.key_pressed(egui::Key::F11) {
                self.is_fullscreen = !self.is_fullscreen;
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.is_fullscreen));
            }

            // Universal Host Escape Keys: Esc, Ctrl+Alt, F12 unlocks both VNC and Capture modes!
            let is_ctrl_alt = i.modifiers.ctrl && i.modifiers.alt;
            if i.key_pressed(egui::Key::Escape) || is_ctrl_alt || i.key_pressed(egui::Key::F12) {
                if self.vnc_is_locked {
                    self.vnc_is_locked = false;
                }
                if self.input_mgr.is_locked() {
                    self.input_mgr.set_locked(false);
                }
            }
        });

        // Main Display Viewport
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::BLACK))
            .show(ctx, |ui| {
                let avail_size = ui.available_size();
                let rect = Rect::from_min_size(ui.cursor().min, avail_size);

                if let Some(ref texture) = self.video_texture {
                    // Render remote screen
                    ui.painter().image(
                        texture.id(),
                        rect,
                        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                } else {
                    // Placeholder screen
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            match self.mode {
                                ClientMode::Vnc => {
                                    if let VncState::Error(ref err) = self.vnc_mgr.get_state() {
                                        ui.colored_label(Color32::from_rgb(255, 90, 90), format!("❌ Connection Error: {}", err));
                                        ui.add_space(12.0);
                                    }

                                    ui.heading("🌐 Native macOS VNC (Screen Sharing)");
                                    ui.add_space(8.0);
                                    ui.label("1. On Mac: System Settings -> General -> Sharing -> Enable 'Screen Sharing'.");
                                    ui.label("2. Click (i) info -> Set 'VNC viewers may control screen with password'.");
                                    ui.label("3. Enter your Mac IP & Password in the top bar, then click 'Connect'.");
                                }
                                ClientMode::CaptureCard => {
                                    ui.heading("📺 Waiting for Mac Mini HDMI Video Feed...");
                                    ui.add_space(8.0);
                                    ui.label("1. Connect HDMI from Mac Mini to your USB Capture Card.");
                                    ui.label("2. Plug USB Capture Card into this PC.");
                                    ui.label("3. Select your device from the dropdown above.");
                                }
                            }
                        });
                    });
                }

                // Interactive Click & Input Handling based on active mode
                match self.mode {
                    ClientMode::Vnc => {
                        if self.vnc_mgr.is_connected() && self.last_vnc_dim.0 > 0 && self.last_vnc_dim.1 > 0 {
                            let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());

                            // Click inside to Lock focus to Mac
                            if response.clicked() && !self.vnc_is_locked {
                                self.vnc_is_locked = true;
                            }

                            if self.vnc_is_locked {
                                // 1. Mouse Position & Click Translation (Deduplicated)
                                if let Some(pos) = response.hover_pos() {
                                    let norm_x = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                                    let norm_y = ((pos.y - rect.top()) / rect.height()).clamp(0.0, 1.0);

                                    let mac_x = (norm_x * (self.last_vnc_dim.0 as f32)) as u16;
                                    let mac_y = (norm_y * (self.last_vnc_dim.1 as f32)) as u16;

                                    let mut button_mask = 0u8;
                                    ctx.input(|i| {
                                        if i.pointer.primary_down() { button_mask |= 1; }   // Left button
                                        if i.pointer.middle_down()  { button_mask |= 2; }   // Middle button
                                        if i.pointer.secondary_down() { button_mask |= 4; } // Right button

                                        // Touchpad & Wheel Scrolling
                                        let scroll_delta = i.smooth_scroll_delta;
                                        if scroll_delta.y > 0.0 {
                                            self.vnc_mgr.send_pointer(button_mask | 8, mac_x, mac_y);  // Wheel Up
                                        } else if scroll_delta.y < 0.0 {
                                            self.vnc_mgr.send_pointer(button_mask | 16, mac_x, mac_y); // Wheel Down
                                        }
                                        if scroll_delta.x > 0.0 {
                                            self.vnc_mgr.send_pointer(button_mask | 32, mac_x, mac_y); // Wheel Left
                                        } else if scroll_delta.x < 0.0 {
                                            self.vnc_mgr.send_pointer(button_mask | 64, mac_x, mac_y); // Wheel Right
                                        }
                                    });

                                    // Only send if pointer or button state actually changed (prevents TCP flooding)
                                    if (mac_x, mac_y, button_mask) != self.last_vnc_pointer {
                                        self.vnc_mgr.send_pointer(button_mask, mac_x, mac_y);
                                        self.last_vnc_pointer = (mac_x, mac_y, button_mask);
                                    }
                                }

                                // 2. Keyboard Input Forwarding
                                ctx.input(|i| {
                                    for event in &i.raw.events {
                                        match event {
                                            egui::Event::Key { key, pressed, modifiers, .. } => {
                                                // Sync modifier keys (Swapped: Alt is Command ⌘, Win is Option ⌥)
                                                if modifiers.alt {
                                                    self.vnc_mgr.send_key(true, keysym::XK_SUPER_L); // Alt -> Command ⌘
                                                } else {
                                                    self.vnc_mgr.send_key(false, keysym::XK_SUPER_L);
                                                }
                                                if modifiers.command {
                                                    self.vnc_mgr.send_key(true, keysym::XK_ALT_L); // Win -> Option ⌥
                                                } else {
                                                    self.vnc_mgr.send_key(false, keysym::XK_ALT_L);
                                                }
                                                if modifiers.ctrl {
                                                    self.vnc_mgr.send_key(true, keysym::XK_CONTROL_L);
                                                } else {
                                                    self.vnc_mgr.send_key(false, keysym::XK_CONTROL_L);
                                                }
                                                if modifiers.shift {
                                                    self.vnc_mgr.send_key(true, keysym::XK_SHIFT_L);
                                                } else {
                                                    self.vnc_mgr.send_key(false, keysym::XK_SHIFT_L);
                                                }

                                                if let Some(keysym) = map_egui_key_to_keysym(*key, modifiers.shift) {
                                                    self.vnc_mgr.send_key(*pressed, keysym);
                                                }
                                            }
                                            egui::Event::Text(text) => {
                                                for c in text.chars() {
                                                    let keysym = c as u32;
                                                    self.vnc_mgr.send_key(true, keysym);
                                                    self.vnc_mgr.send_key(false, keysym);
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                });
                            }
                        }
                    }

                    ClientMode::CaptureCard => {
                        let response = ui.allocate_rect(rect, egui::Sense::click());

                        // Locking with nowhere to send input just swallows the
                        // mouse and looks like a freeze, so gate it on the link.
                        if self.network.is_connected() {
                            if response.clicked() && !is_capture_locked {
                                self.input_mgr.set_locked(true);
                            }
                        } else if self.video_texture.is_some() {
                            let hint = if self.network.is_discovering() {
                                "🔍 Searching the network for MacReceiver..."
                            } else {
                                "⚠ Not linked - enter the Mac IP and click Link to control the Mac"
                            };
                            ui.painter().text(
                                egui::pos2(rect.center().x, rect.bottom() - 24.0),
                                egui::Align2::CENTER_BOTTOM,
                                hint,
                                egui::FontId::proportional(15.0),
                                Color32::from_rgb(255, 200, 90),
                            );
                        }
                    }
                }
            });
    }
}

/// Translates an egui::Key and Shift state to an X11/RFB KeySym
fn map_egui_key_to_keysym(key: egui::Key, is_shift: bool) -> Option<u32> {
    match key {
        egui::Key::A => Some(if is_shift { b'A' as u32 } else { b'a' as u32 }),
        egui::Key::B => Some(if is_shift { b'B' as u32 } else { b'b' as u32 }),
        egui::Key::C => Some(if is_shift { b'C' as u32 } else { b'c' as u32 }),
        egui::Key::D => Some(if is_shift { b'D' as u32 } else { b'd' as u32 }),
        egui::Key::E => Some(if is_shift { b'E' as u32 } else { b'e' as u32 }),
        egui::Key::F => Some(if is_shift { b'F' as u32 } else { b'f' as u32 }),
        egui::Key::G => Some(if is_shift { b'G' as u32 } else { b'g' as u32 }),
        egui::Key::H => Some(if is_shift { b'H' as u32 } else { b'h' as u32 }),
        egui::Key::I => Some(if is_shift { b'I' as u32 } else { b'i' as u32 }),
        egui::Key::J => Some(if is_shift { b'J' as u32 } else { b'j' as u32 }),
        egui::Key::K => Some(if is_shift { b'K' as u32 } else { b'k' as u32 }),
        egui::Key::L => Some(if is_shift { b'L' as u32 } else { b'l' as u32 }),
        egui::Key::M => Some(if is_shift { b'M' as u32 } else { b'm' as u32 }),
        egui::Key::N => Some(if is_shift { b'N' as u32 } else { b'n' as u32 }),
        egui::Key::O => Some(if is_shift { b'O' as u32 } else { b'o' as u32 }),
        egui::Key::P => Some(if is_shift { b'P' as u32 } else { b'p' as u32 }),
        egui::Key::Q => Some(if is_shift { b'Q' as u32 } else { b'q' as u32 }),
        egui::Key::R => Some(if is_shift { b'R' as u32 } else { b'r' as u32 }),
        egui::Key::S => Some(if is_shift { b'S' as u32 } else { b's' as u32 }),
        egui::Key::T => Some(if is_shift { b'T' as u32 } else { b't' as u32 }),
        egui::Key::U => Some(if is_shift { b'U' as u32 } else { b'u' as u32 }),
        egui::Key::V => Some(if is_shift { b'V' as u32 } else { b'v' as u32 }),
        egui::Key::W => Some(if is_shift { b'W' as u32 } else { b'w' as u32 }),
        egui::Key::X => Some(if is_shift { b'X' as u32 } else { b'x' as u32 }),
        egui::Key::Y => Some(if is_shift { b'Y' as u32 } else { b'y' as u32 }),
        egui::Key::Z => Some(if is_shift { b'Z' as u32 } else { b'z' as u32 }),

        egui::Key::Num0 => Some(if is_shift { b')' as u32 } else { b'0' as u32 }),
        egui::Key::Num1 => Some(if is_shift { b'!' as u32 } else { b'1' as u32 }),
        egui::Key::Num2 => Some(if is_shift { b'@' as u32 } else { b'2' as u32 }),
        egui::Key::Num3 => Some(if is_shift { b'#' as u32 } else { b'3' as u32 }),
        egui::Key::Num4 => Some(if is_shift { b'$' as u32 } else { b'4' as u32 }),
        egui::Key::Num5 => Some(if is_shift { b'%' as u32 } else { b'5' as u32 }),
        egui::Key::Num6 => Some(if is_shift { b'^' as u32 } else { b'6' as u32 }),
        egui::Key::Num7 => Some(if is_shift { b'&' as u32 } else { b'7' as u32 }),
        egui::Key::Num8 => Some(if is_shift { b'*' as u32 } else { b'8' as u32 }),
        egui::Key::Num9 => Some(if is_shift { b'(' as u32 } else { b'9' as u32 }),

        egui::Key::Backspace => Some(keysym::XK_BACKSPACE),
        egui::Key::Tab => Some(keysym::XK_TAB),
        egui::Key::Enter => Some(keysym::XK_RETURN),
        egui::Key::Escape => Some(keysym::XK_ESCAPE),
        egui::Key::Space => Some(b' ' as u32),
        egui::Key::Delete => Some(keysym::XK_DELETE),

        egui::Key::ArrowLeft => Some(keysym::XK_LEFT),
        egui::Key::ArrowUp => Some(keysym::XK_UP),
        egui::Key::ArrowRight => Some(keysym::XK_RIGHT),
        egui::Key::ArrowDown => Some(keysym::XK_DOWN),
        egui::Key::Home => Some(keysym::XK_HOME),
        egui::Key::End => Some(keysym::XK_END),
        egui::Key::PageUp => Some(keysym::XK_PAGE_UP),
        egui::Key::PageDown => Some(keysym::XK_PAGE_DOWN),

        egui::Key::F1 => Some(keysym::XK_F1),
        egui::Key::F2 => Some(keysym::XK_F2),
        egui::Key::F3 => Some(keysym::XK_F3),
        egui::Key::F4 => Some(keysym::XK_F4),
        egui::Key::F5 => Some(keysym::XK_F5),
        egui::Key::F6 => Some(keysym::XK_F6),
        egui::Key::F7 => Some(keysym::XK_F7),
        egui::Key::F8 => Some(keysym::XK_F8),
        egui::Key::F9 => Some(keysym::XK_F9),
        egui::Key::F10 => Some(keysym::XK_F10),
        egui::Key::F11 => Some(keysym::XK_F11),
        egui::Key::F12 => Some(keysym::XK_F12),

        _ => None,
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("MacConnect - macOS VNC & Remote Connect")
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "MacConnect",
        native_options,
        Box::new(|cc| Ok(Box::new(MacConnectApp::new(cc)))),
    )
}
