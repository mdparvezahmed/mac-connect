use nokhwa::pixel_format::RgbAFormat;
use nokhwa::utils::{ApiBackend, CameraIndex, RequestedFormat, RequestedFormatType};
use nokhwa::Camera;

fn main() {
    println!("=== Testing NV12 Decoding on UGREEN 25854 (Index 1) ===");
    let idx = CameraIndex::Index(1);
    let req = RequestedFormat::new::<RgbAFormat>(RequestedFormatType::None);
    match Camera::new(idx, req) {
        Ok(mut camera) => {
            println!("Camera format: {:?}", camera.camera_format());
            if let Err(e) = camera.open_stream() {
                println!("Open stream error: {:?}", e);
                return;
            }

            match camera.frame() {
                Ok(frame) => {
                    println!("Frame received: {} bytes, format: {:?}", frame.buffer().len(), frame.source_frame_format());
                    match frame.decode_image::<RgbAFormat>() {
                        Ok(img) => {
                            println!("✓ Successfully decoded RGBA frame! Dimensions: {}x{}, raw size: {} bytes", img.width(), img.height(), img.into_raw().len());
                        }
                        Err(e) => {
                            println!("✗ Decode image error: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("Frame read error: {:?}", e);
                }
            }
            let _ = camera.stop_stream();
        }
        Err(e) => {
            println!("Create camera error: {:?}", e);
        }
    }
}
