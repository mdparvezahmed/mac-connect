use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use super::des::vnc_encrypt_challenge;

#[derive(Clone, Debug)]
pub struct VncServerInfo {
    pub width: u16,
    pub height: u16,
    pub name: String,
}

pub struct RfbHandshakeResult {
    pub read_stream: TcpStream,
    pub write_stream: TcpStream,
    pub info: VncServerInfo,
}

impl RfbHandshakeResult {
    pub fn connect(host: &str, port: u16, password: &str) -> Result<Self, String> {
        let addr = format!("{}:{}", host.trim(), port);
        let mut stream = TcpStream::connect(&addr)
            .map_err(|e| format!("Could not connect to {}: {}", addr, e))?;

        stream.set_nodelay(true).map_err(|e| e.to_string())?;
        stream.set_read_timeout(Some(Duration::from_secs(10))).map_err(|e| e.to_string())?;
        stream.set_write_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;

        // 1. Version Handshake
        let mut version_buf = [0u8; 12];
        stream.read_exact(&mut version_buf)
            .map_err(|e| format!("Failed to read RFB version: {}", e))?;

        let server_version = String::from_utf8_lossy(&version_buf);
        if !server_version.starts_with("RFB ") {
            return Err(format!("Invalid RFB protocol header: {}", server_version));
        }

        // Send RFB 003.008\n
        stream.write_all(b"RFB 003.008\n")
            .map_err(|e| format!("Failed to send RFB version: {}", e))?;

        // 2. Security Handshake
        let num_security_types = stream.read_u8()
            .map_err(|e| format!("Failed to read security types count: {}", e))?;

        if num_security_types == 0 {
            let reason_len = stream.read_u32::<BigEndian>().unwrap_or(0);
            let mut reason = vec![0u8; reason_len as usize];
            let _ = stream.read_exact(&mut reason);
            return Err(format!("Server rejected connection: {}", String::from_utf8_lossy(&reason)));
        }

        let mut security_types = vec![0u8; num_security_types as usize];
        stream.read_exact(&mut security_types)
            .map_err(|e| format!("Failed to read security types: {}", e))?;

        let mut chosen_security: Option<u8> = None;
        if security_types.contains(&2) {
            chosen_security = Some(2);
        } else if security_types.contains(&1) {
            chosen_security = Some(1);
        }

        let sec_type = chosen_security
            .ok_or_else(|| format!("No supported security type found on server: {:?}", security_types))?;

        stream.write_u8(sec_type)
            .map_err(|e| format!("Failed to send chosen security type: {}", e))?;

        // Handle VNC Authentication (Type 2)
        if sec_type == 2 {
            let mut challenge = [0u8; 16];
            stream.read_exact(&mut challenge)
                .map_err(|e| format!("Failed to read authentication challenge: {}", e))?;

            let response = vnc_encrypt_challenge(&challenge, password);
            stream.write_all(&response)
                .map_err(|e| format!("Failed to send authentication response: {}", e))?;

            let auth_result = stream.read_u32::<BigEndian>()
                .map_err(|e| format!("Failed to read auth result: {}", e))?;

            if auth_result != 0 {
                let mut reason_str = String::new();
                if let Ok(reason_len) = stream.read_u32::<BigEndian>() {
                    let mut reason = vec![0u8; reason_len as usize];
                    if stream.read_exact(&mut reason).is_ok() {
                        reason_str = format!(": {}", String::from_utf8_lossy(&reason));
                    }
                }
                return Err(format!("VNC Authentication failed (wrong password?){}", reason_str));
            }
        }

        // 3. ClientInit (Shared desktop = 1)
        stream.write_u8(1).map_err(|e| e.to_string())?;

        // 4. ServerInit
        let width = stream.read_u16::<BigEndian>().map_err(|e| e.to_string())?;
        let height = stream.read_u16::<BigEndian>().map_err(|e| e.to_string())?;

        // Pixel format (16 bytes)
        let mut _pix_fmt = [0u8; 16];
        stream.read_exact(&mut _pix_fmt).map_err(|e| e.to_string())?;

        let name_len = stream.read_u32::<BigEndian>().map_err(|e| e.to_string())? as usize;
        let mut name_bytes = vec![0u8; name_len];
        stream.read_exact(&mut name_bytes).map_err(|e| e.to_string())?;
        let name = String::from_utf8_lossy(&name_bytes).to_string();

        let info = VncServerInfo {
            width,
            height,
            name,
        };

        // 5. Configure Pixel Format: 32-bit RGBA (Red shift 0, Green shift 8, Blue shift 16)
        let mut fmt_buf = Vec::with_capacity(20);
        fmt_buf.push(0); // Message type: SetPixelFormat
        fmt_buf.push(0); // Pad
        fmt_buf.push(0);
        fmt_buf.push(0);
        fmt_buf.push(32); // Bits-per-pixel: 32
        fmt_buf.push(24); // Depth: 24
        fmt_buf.push(0);  // Big-endian-flag: 0 (Little-endian)
        fmt_buf.push(1);  // True-colour-flag: 1
        fmt_buf.write_u16::<BigEndian>(255).unwrap(); // Red-max
        fmt_buf.write_u16::<BigEndian>(255).unwrap(); // Green-max
        fmt_buf.write_u16::<BigEndian>(255).unwrap(); // Blue-max
        fmt_buf.push(0);  // Red-shift: 0
        fmt_buf.push(8);  // Green-shift: 8
        fmt_buf.push(16); // Blue-shift: 16
        fmt_buf.push(0);  // Pad
        fmt_buf.push(0);
        fmt_buf.push(0);
        stream.write_all(&fmt_buf).map_err(|e| e.to_string())?;

        // 6. Set Encodings: Raw (0), CopyRect (1), DesktopSize (-223), Cursor (-239), LastRect (-224)
        let encodings = [0i32, 1, -223, -239, -224];
        let mut enc_buf = Vec::with_capacity(4 + encodings.len() * 4);
        enc_buf.push(2); // Message type: SetEncodings
        enc_buf.push(0); // Pad
        enc_buf.write_u16::<BigEndian>(encodings.len() as u16).unwrap();
        for &enc in &encodings {
            enc_buf.write_i32::<BigEndian>(enc).unwrap();
        }
        stream.write_all(&enc_buf).map_err(|e| e.to_string())?;

        // 7. Clone stream for full-duplex reader & writer
        let write_stream = stream.try_clone().map_err(|e| e.to_string())?;
        let read_stream = stream;

        // Long timeout for reader so large frames never timeout
        read_stream.set_read_timeout(Some(Duration::from_secs(15))).map_err(|e| e.to_string())?;

        Ok(Self {
            read_stream,
            write_stream,
            info,
        })
    }
}
