#![allow(dead_code)]

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use crossbeam_channel::{Receiver, Sender};
use nokhwa::pixel_format::RgbAFormat;
use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};
use nokhwa::Camera;
use parking_lot::RwLock;

pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>, // RGBA format
}

#[derive(Clone, Debug)]
pub struct VideoDeviceInfo {
    pub index: u32,
    pub name: String,
    pub description: String,
}

pub struct VideoCaptureManager {
    running: Arc<AtomicBool>,
    active_device_index: Arc<AtomicU32>,
    frame_sender: Sender<VideoFrame>,
    frame_receiver: Receiver<VideoFrame>,
    is_capturing: Arc<AtomicBool>,
    last_error: Arc<RwLock<Option<String>>>,
}

const NO_DEVICE: u32 = u32::MAX;

impl VideoCaptureManager {
    pub fn new() -> Self {
        let (frame_sender, frame_receiver) = crossbeam_channel::bounded(2);
        let mgr = Self {
            running: Arc::new(AtomicBool::new(true)),
            active_device_index: Arc::new(AtomicU32::new(NO_DEVICE)),
            frame_sender,
            frame_receiver,
            is_capturing: Arc::new(AtomicBool::new(false)),
            last_error: Arc::new(RwLock::new(None)),
        };

        mgr.start_worker_thread();
        mgr
    }

    pub fn get_frame_receiver(&self) -> Receiver<VideoFrame> {
        self.frame_receiver.clone()
    }

    pub fn is_capturing(&self) -> bool {
        self.is_capturing.load(Ordering::Relaxed)
    }

    pub fn get_last_error(&self) -> Option<String> {
        self.last_error.read().clone()
    }

    pub fn list_devices() -> Vec<VideoDeviceInfo> {
        let mut devices = Vec::new();
        if let Ok(cams) = nokhwa::query(nokhwa::utils::ApiBackend::MediaFoundation) {
            for (i, cam) in cams.into_iter().enumerate() {
                devices.push(VideoDeviceInfo {
                    index: i as u32,
                    name: cam.human_name(),
                    description: cam.description().to_string(),
                });
            }
        }
        devices
    }

    pub fn start_capture(&self, device_index: u32) {
        *self.last_error.write() = None;
        self.active_device_index.store(device_index, Ordering::SeqCst);
    }

    pub fn stop_capture(&self) {
        self.active_device_index.store(NO_DEVICE, Ordering::SeqCst);
    }

    fn start_worker_thread(&self) {
        let running = self.running.clone();
        let active_device_index = self.active_device_index.clone();
        let frame_sender = self.frame_sender.clone();
        let is_capturing = self.is_capturing.clone();
        let last_error = self.last_error.clone();

        std::thread::spawn(move || {
            let mut current_open_index = NO_DEVICE;
            let mut active_camera: Option<Camera> = None;

            while running.load(Ordering::Relaxed) {
                let target_index = active_device_index.load(Ordering::SeqCst);

                // If target index changed, close previous camera and open new one
                if target_index != current_open_index {
                    if let Some(mut cam) = active_camera.take() {
                        let _ = cam.stop_stream();
                    }
                    is_capturing.store(false, Ordering::SeqCst);
                    current_open_index = NO_DEVICE;

                    if target_index != NO_DEVICE {
                        let idx = CameraIndex::Index(target_index);
                        let req = RequestedFormat::new::<RgbAFormat>(RequestedFormatType::None);

                        match Camera::new(idx, req) {
                            Ok(mut cam) => {
                                match cam.open_stream() {
                                    Ok(_) => {
                                        active_camera = Some(cam);
                                        current_open_index = target_index;
                                        is_capturing.store(true, Ordering::SeqCst);
                                        *last_error.write() = None;
                                    }
                                    Err(e) => {
                                        let err_msg = format!("Failed to open video stream: {:?}", e);
                                        *last_error.write() = Some(err_msg);
                                    }
                                }
                            }
                            Err(e) => {
                                let err_msg = format!("Failed to initialize camera device: {:?}", e);
                                *last_error.write() = Some(err_msg);
                            }
                        }
                    }
                }

                // If camera is open and active, read frame
                if let Some(ref mut cam) = active_camera {
                    match cam.frame() {
                        Ok(frame_buf) => {
                            match frame_buf.decode_image::<RgbAFormat>() {
                                Ok(image) => {
                                    let (w, h) = (image.width(), image.height());
                                    let frame = VideoFrame {
                                        width: w,
                                        height: h,
                                        data: image.into_raw(),
                                    };
                                    let _ = frame_sender.try_send(frame);
                                }
                                Err(e) => {
                                    eprintln!("Decode frame error: {:?}", e);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Frame capture error: {:?}", e);
                            std::thread::sleep(Duration::from_millis(5));
                        }
                    }
                } else {
                    // Idle sleep when no camera is active
                    std::thread::sleep(Duration::from_millis(20));
                }
            }

            if let Some(mut cam) = active_camera.take() {
                let _ = cam.stop_stream();
            }
        });
    }
}
