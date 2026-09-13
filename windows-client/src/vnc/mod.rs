pub mod des;
pub mod keysym;
pub mod rfb;

use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use crossbeam_channel::{Receiver, Sender};
use parking_lot::RwLock;

use crate::video::VideoFrame;
use rfb::RfbHandshakeResult;

#[derive(Clone, Debug, PartialEq)]
pub enum VncState {
    Disconnected,
    Connecting,
    Connected { width: u16, height: u16, name: String },
    Error(String),
}

pub enum VncCommand {
    Pointer { button_mask: u8, x: u16, y: u16 },
    Key { is_down: bool, keysym: u32 },
    RequestUpdate { incremental: u8, x: u16, y: u16, w: u16, h: u16 },
}

pub struct VncManager {
    state: Arc<RwLock<VncState>>,
    running: Arc<AtomicBool>,
    cmd_sender: Sender<VncCommand>,
    cmd_receiver: Receiver<VncCommand>,
    frame_sender: Sender<VideoFrame>,
    frame_receiver: Receiver<VideoFrame>,
}

impl VncManager {
    pub fn new() -> Self {
        let (cmd_sender, cmd_receiver) = crossbeam_channel::bounded(512);
        let (frame_sender, frame_receiver) = crossbeam_channel::bounded(4);

        Self {
            state: Arc::new(RwLock::new(VncState::Disconnected)),
            running: Arc::new(AtomicBool::new(false)),
            cmd_sender,
            cmd_receiver,
            frame_sender,
            frame_receiver,
        }
    }

    pub fn get_state(&self) -> VncState {
        self.state.read().clone()
    }

    pub fn is_connected(&self) -> bool {
        matches!(*self.state.read(), VncState::Connected { .. })
    }

    pub fn get_frame_receiver(&self) -> Receiver<VideoFrame> {
        self.frame_receiver.clone()
    }

    pub fn connect(&self, host: String, port: u16, password: String) {
        self.disconnect();

        *self.state.write() = VncState::Connecting;
        self.running.store(true, Ordering::SeqCst);

        let state = self.state.clone();
        let running = self.running.clone();
        let cmd_tx = self.cmd_sender.clone();
        let cmd_rx = self.cmd_receiver.clone();
        let frame_tx = self.frame_sender.clone();

        std::thread::spawn(move || {
            let handshake = match RfbHandshakeResult::connect(&host, port, &password) {
                Ok(h) => h,
                Err(e) => {
                    *state.write() = VncState::Error(e);
                    running.store(false, Ordering::SeqCst);
                    return;
                }
            };

            let mut width = handshake.info.width;
            let mut height = handshake.info.height;

            *state.write() = VncState::Connected {
                width,
                height,
                name: handshake.info.name.clone(),
            };

            let mut write_stream = handshake.write_stream;
            let mut read_stream = handshake.read_stream;

            // 1. Spawn Writer Thread for instant, non-blocking input and update requests
            let writer_running = running.clone();
            std::thread::spawn(move || {
                while writer_running.load(Ordering::Relaxed) {
                    match cmd_rx.recv_timeout(Duration::from_millis(50)) {
                        Ok(VncCommand::Pointer { button_mask, x, y }) => {
                            let mut buf = [0u8; 6];
                            buf[0] = 5; // PointerEvent
                            buf[1] = button_mask;
                            (&mut buf[2..4]).write_u16::<BigEndian>(x).unwrap();
                            (&mut buf[4..6]).write_u16::<BigEndian>(y).unwrap();
                            if write_stream.write_all(&buf).is_err() {
                                break;
                            }
                        }
                        Ok(VncCommand::Key { is_down, keysym }) => {
                            let mut buf = [0u8; 8];
                            buf[0] = 4; // KeyEvent
                            buf[1] = if is_down { 1 } else { 0 };
                            buf[2] = 0;
                            buf[3] = 0;
                            (&mut buf[4..8]).write_u32::<BigEndian>(keysym).unwrap();
                            if write_stream.write_all(&buf).is_err() {
                                break;
                            }
                        }
                        Ok(VncCommand::RequestUpdate { incremental, x, y, w, h }) => {
                            let mut buf = [0u8; 10];
                            buf[0] = 3; // FramebufferUpdateRequest
                            buf[1] = incremental;
                            (&mut buf[2..4]).write_u16::<BigEndian>(x).unwrap();
                            (&mut buf[4..6]).write_u16::<BigEndian>(y).unwrap();
                            (&mut buf[6..8]).write_u16::<BigEndian>(w).unwrap();
                            (&mut buf[8..10]).write_u16::<BigEndian>(h).unwrap();
                            if write_stream.write_all(&buf).is_err() {
                                break;
                            }
                        }
                        Err(_) => {}
                    }
                }
            });

            // 2. Initial Full Framebuffer Update Request
            let _ = cmd_tx.try_send(VncCommand::RequestUpdate {
                incremental: 0,
                x: 0,
                y: 0,
                w: width,
                h: height,
            });

            let mut frame_buffer = vec![255u8; (width as usize) * (height as usize) * 4];

            // 3. Reader Loop for streaming incoming video updates
            while running.load(Ordering::Relaxed) {
                let mut msg_type = [0u8; 1];
                if let Err(e) = read_stream.read_exact(&mut msg_type) {
                    if running.load(Ordering::Relaxed) {
                        *state.write() = VncState::Error(format!("Connection closed: {}", e));
                    }
                    break;
                }

                match msg_type[0] {
                    0 => {
                        // FramebufferUpdate
                        let mut pad = [0u8; 1];
                        if read_stream.read_exact(&mut pad).is_err() { break; }
                        let num_rects = match read_stream.read_u16::<BigEndian>() {
                            Ok(n) => n,
                            Err(_) => break,
                        };

                        let mut has_update = false;

                        for _ in 0..num_rects {
                            let rx = match read_stream.read_u16::<BigEndian>() { Ok(v) => v, Err(_) => break };
                            let ry = match read_stream.read_u16::<BigEndian>() { Ok(v) => v, Err(_) => break };
                            let rw = match read_stream.read_u16::<BigEndian>() { Ok(v) => v, Err(_) => break };
                            let rh = match read_stream.read_u16::<BigEndian>() { Ok(v) => v, Err(_) => break };
                            let encoding = match read_stream.read_i32::<BigEndian>() { Ok(v) => v, Err(_) => break };

                            match encoding {
                                0 => {
                                    // Raw Encoding (32bpp)
                                    let rect_bytes_len = (rw as usize) * (rh as usize) * 4;
                                    let mut rect_data = vec![0u8; rect_bytes_len];
                                    if read_stream.read_exact(&mut rect_data).is_err() { break; }

                                    // Ensure alpha is fully opaque (255)
                                    for px in rect_data.chunks_exact_mut(4) {
                                        px[3] = 255;
                                    }

                                    let fb_width = width as usize;
                                    for row in 0..(rh as usize) {
                                        let dst_y = (ry as usize) + row;
                                        if dst_y >= height as usize { break; }

                                        let dst_start = (dst_y * fb_width + (rx as usize)) * 4;
                                        let src_start = (row * (rw as usize)) * 4;
                                        let row_len = (rw as usize).min(fb_width.saturating_sub(rx as usize)) * 4;

                                        frame_buffer[dst_start..dst_start + row_len]
                                            .copy_from_slice(&rect_data[src_start..src_start + row_len]);
                                    }
                                    has_update = true;
                                }
                                1 => {
                                    // CopyRect Encoding
                                    let src_x = match read_stream.read_u16::<BigEndian>() { Ok(v) => v, Err(_) => break };
                                    let src_y = match read_stream.read_u16::<BigEndian>() { Ok(v) => v, Err(_) => break };

                                    let fb_width = width as usize;
                                    for row in 0..(rh as usize) {
                                        let sy = (src_y as usize) + row;
                                        let dy = (ry as usize) + row;
                                        if sy >= height as usize || dy >= height as usize { break; }

                                        let src_start = (sy * fb_width + (src_x as usize)) * 4;
                                        let dst_start = (dy * fb_width + (rx as usize)) * 4;
                                        let len = (rw as usize) * 4;

                                        frame_buffer.copy_within(src_start..src_start + len, dst_start);
                                    }
                                    has_update = true;
                                }
                                -223 => {
                                    // DesktopSize Pseudo-encoding
                                    width = rw;
                                    height = rh;
                                    frame_buffer = vec![255u8; (rw as usize) * (rh as usize) * 4];
                                    has_update = true;
                                }
                                -239 => {
                                    // Cursor Pseudo-encoding
                                    let data_len = (rw as usize) * (rh as usize) * 4 + (((rw as usize + 7) / 8) * (rh as usize));
                                    if data_len > 0 {
                                        let mut dummy = vec![0u8; data_len];
                                        let _ = read_stream.read_exact(&mut dummy);
                                    }
                                }
                                -224 => {
                                    // LastRect
                                    break;
                                }
                                _ => {
                                    eprintln!("Unsupported RFB encoding: {}", encoding);
                                }
                            }
                        }

                        if has_update {
                            let frame = VideoFrame {
                                width: width as u32,
                                height: height as u32,
                                data: frame_buffer.clone(),
                            };
                            let _ = frame_tx.try_send(frame);
                        }

                        // Immediately request next incremental update
                        let _ = cmd_tx.try_send(VncCommand::RequestUpdate {
                            incremental: 1,
                            x: 0,
                            y: 0,
                            w: width,
                            h: height,
                        });
                    }
                    3 => {
                        // ServerCutText
                        let mut pad = [0u8; 3];
                        let _ = read_stream.read_exact(&mut pad);
                        if let Ok(len) = read_stream.read_u32::<BigEndian>() {
                            let mut text_buf = vec![0u8; len as usize];
                            let _ = read_stream.read_exact(&mut text_buf);
                        }
                    }
                    _other => {}
                }
            }

            running.store(false, Ordering::SeqCst);
            if !matches!(*state.read(), VncState::Error(_)) {
                *state.write() = VncState::Disconnected;
            }
        });
    }

    pub fn disconnect(&self) {
        self.running.store(false, Ordering::SeqCst);
        *self.state.write() = VncState::Disconnected;
    }

    pub fn send_pointer(&self, button_mask: u8, x: u16, y: u16) {
        let _ = self.cmd_sender.try_send(VncCommand::Pointer { button_mask, x, y });
    }

    pub fn send_key(&self, is_down: bool, keysym: u32) {
        let _ = self.cmd_sender.try_send(VncCommand::Key { is_down, keysym });
    }
}
