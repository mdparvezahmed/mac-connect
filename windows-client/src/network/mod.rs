use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use parking_lot::RwLock;

use crate::protocol::{PacketSerializer, MAGIC0, MAGIC1, PacketType};

#[derive(Clone)]
pub struct NetworkClient {
    socket: Arc<UdpSocket>,
    target_addr: Arc<RwLock<Option<SocketAddr>>>,
    last_rtt_ms: Arc<AtomicU64>,
    is_connected: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
}

impl NetworkClient {
    pub fn new() -> std::io::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_nonblocking(true)?;

        let client = Self {
            socket: Arc::new(socket),
            target_addr: Arc::new(RwLock::new(None)),
            last_rtt_ms: Arc::new(AtomicU64::new(0)),
            is_connected: Arc::new(AtomicBool::new(false)),
            running: Arc::new(AtomicBool::new(true)),
        };

        // Start ping/pong & receive thread
        client.start_background_worker();

        Ok(client)
    }

    pub fn set_target(&self, ip_str: &str, port: u16) -> bool {
        let addr_str = format!("{}:{}", ip_str.trim(), port);
        if let Ok(addr) = addr_str.parse::<SocketAddr>() {
            *self.target_addr.write() = Some(addr);
            self.is_connected.store(false, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    pub fn send(&self, data: &[u8]) {
        if let Some(target) = *self.target_addr.read() {
            let _ = self.socket.send_to(data, target);
        }
    }

    pub fn get_rtt_ms(&self) -> u64 {
        self.last_rtt_ms.load(Ordering::Relaxed)
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::Relaxed)
    }

    fn start_background_worker(&self) {
        let socket = self.socket.clone();
        let target_addr = self.target_addr.clone();
        let last_rtt_ms = self.last_rtt_ms.clone();
        let is_connected = self.is_connected.clone();
        let running = self.running.clone();

        std::thread::spawn(move || {
            let mut buf = [0u8; 1024];
            let mut last_ping = Instant::now();

            while running.load(Ordering::Relaxed) {
                // Send Ping every 1 second if target is set
                if last_ping.elapsed() >= Duration::from_millis(1000) {
                    last_ping = Instant::now();
                    if let Some(target) = *target_addr.read() {
                        let now_millis = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;
                        let ping_pkt = PacketSerializer::ping(now_millis);
                        let _ = socket.send_to(&ping_pkt, target);
                    }
                }

                // Try receiving Pong packet
                match socket.recv_from(&mut buf) {
                    Ok((n, _from_addr)) => {
                        if n >= 11 && buf[0] == MAGIC0 && buf[1] == MAGIC1 && buf[2] == PacketType::Pong as u8 {
                            let mut ts_bytes = [0u8; 8];
                            ts_bytes.copy_from_slice(&buf[3..11]);
                            let sent_ts = u64::from_le_bytes(ts_bytes);

                            let now_millis = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64;
                            let rtt = now_millis.saturating_sub(sent_ts);
                            last_rtt_ms.store(rtt, Ordering::Relaxed);
                            is_connected.store(true, Ordering::Relaxed);
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                }
            }
        });
    }
}
