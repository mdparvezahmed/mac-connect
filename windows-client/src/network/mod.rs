#![allow(dead_code)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU64, Ordering};
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
    /// Port probed by auto-discovery.
    discovery_port: Arc<AtomicU16>,
    /// True while we should keep broadcasting to find a receiver on the LAN.
    auto_discover: Arc<AtomicBool>,
    /// Set when the current target came from discovery rather than the user,
    /// so we are free to drop it and search again if the Mac disappears.
    target_is_auto: Arc<AtomicBool>,
}

impl NetworkClient {
    pub fn new(default_port: u16) -> std::io::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_nonblocking(true)?;
        // Needed to probe the LAN for a receiver; harmless for unicast traffic.
        let _ = socket.set_broadcast(true);

        let client = Self {
            socket: Arc::new(socket),
            target_addr: Arc::new(RwLock::new(None)),
            last_rtt_ms: Arc::new(AtomicU64::new(0)),
            is_connected: Arc::new(AtomicBool::new(false)),
            running: Arc::new(AtomicBool::new(true)),
            discovery_port: Arc::new(AtomicU16::new(default_port)),
            auto_discover: Arc::new(AtomicBool::new(true)),
            target_is_auto: Arc::new(AtomicBool::new(false)),
        };

        // Start ping/pong & receive thread
        client.start_background_worker();

        Ok(client)
    }

    pub fn set_target(&self, ip_str: &str, port: u16) -> bool {
        let addr_str = format!("{}:{}", ip_str.trim(), port);
        if let Ok(addr) = addr_str.parse::<SocketAddr>() {
            *self.target_addr.write() = Some(addr);
            self.target_is_auto.store(false, Ordering::Relaxed);
            self.discovery_port.store(port, Ordering::Relaxed);
            self.is_connected.store(false, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Address we are currently talking to, if any.
    pub fn target_ip(&self) -> Option<String> {
        self.target_addr.read().map(|a| a.ip().to_string())
    }

    /// Whether we are still hunting the LAN for a receiver.
    pub fn is_discovering(&self) -> bool {
        self.auto_discover.load(Ordering::Relaxed) && self.target_addr.read().is_none()
    }

    /// Turns LAN auto-discovery on or off. While on, the client broadcasts a
    /// ping once a second and adopts whichever host answers with a pong.
    pub fn set_auto_discover(&self, enabled: bool, port: u16) {
        self.discovery_port.store(port, Ordering::Relaxed);
        self.auto_discover.store(enabled, Ordering::Relaxed);
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

    pub fn disconnect(&self) {
        *self.target_addr.write() = None;
        self.target_is_auto.store(false, Ordering::Relaxed);
        self.is_connected.store(false, Ordering::Relaxed);
        // An explicit unlink is the user taking over: stop re-adopting the Mac
        // on the next broadcast, or the link would spring straight back.
        self.auto_discover.store(false, Ordering::Relaxed);
    }

    fn start_background_worker(&self) {
        let socket = self.socket.clone();
        let target_addr = self.target_addr.clone();
        let last_rtt_ms = self.last_rtt_ms.clone();
        let is_connected = self.is_connected.clone();
        let running = self.running.clone();
        let discovery_port = self.discovery_port.clone();
        let auto_discover = self.auto_discover.clone();
        let target_is_auto = self.target_is_auto.clone();

        std::thread::spawn(move || {
            let mut buf = [0u8; 1024];
            let mut last_ping = Instant::now() - Duration::from_secs(1);
            let mut last_pong: Option<Instant> = None;

            while running.load(Ordering::Relaxed) {
                // Ping the target once a second, or - when we have none - shout
                // across the LAN so a listening receiver can answer and identify
                // itself. The receiver already pongs any ping, so discovery needs
                // no extra protocol support on the Mac side.
                if last_ping.elapsed() >= Duration::from_millis(1000) {
                    last_ping = Instant::now();

                    let now_millis = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    let ping_pkt = PacketSerializer::ping(now_millis);

                    let current_target = *target_addr.read();
                    match current_target {
                        Some(target) => {
                            let _ = socket.send_to(&ping_pkt, target);
                        }
                        None => {
                            if auto_discover.load(Ordering::Relaxed) {
                                let port = discovery_port.load(Ordering::Relaxed);
                                for addr in broadcast_targets(port) {
                                    let _ = socket.send_to(&ping_pkt, addr);
                                }
                            }
                        }
                    }
                }

                // Drop a link that has gone quiet, so the UI stops claiming we
                // are connected and discovery can pick the Mac up again.
                if let Some(seen) = last_pong {
                    if seen.elapsed() >= Duration::from_secs(3) {
                        last_pong = None;
                        is_connected.store(false, Ordering::Relaxed);
                        if target_is_auto.load(Ordering::Relaxed) {
                            *target_addr.write() = None;
                        }
                    }
                }

                // Try receiving Pong packet
                match socket.recv_from(&mut buf) {
                    Ok((n, from_addr)) => {
                        if n >= 11 && buf[0] == MAGIC0 && buf[1] == MAGIC1 && buf[2] == PacketType::Pong as u8 {
                            let mut ts_bytes = [0u8; 8];
                            ts_bytes.copy_from_slice(&buf[3..11]);
                            let sent_ts = u64::from_le_bytes(ts_bytes);

                            let now_millis = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64;
                            // A pong while we have no target means a broadcast
                            // found the Mac: adopt whoever answered. Only while
                            // discovery is on, so a pong still in flight cannot
                            // undo an explicit unlink.
                            let mut have_target = target_addr.read().is_some();
                            if !have_target && auto_discover.load(Ordering::Relaxed) {
                                *target_addr.write() = Some(from_addr);
                                target_is_auto.store(true, Ordering::Relaxed);
                                have_target = true;
                            }

                            if have_target {
                                let rtt = now_millis.saturating_sub(sent_ts);
                                last_rtt_ms.store(rtt, Ordering::Relaxed);
                                is_connected.store(true, Ordering::Relaxed);
                                last_pong = Some(Instant::now());
                            }
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

/// Addresses worth probing when looking for a receiver on the local network.
///
/// Limited broadcast (255.255.255.255) is dropped by some adapters and virtual
/// switches, so we also aim at the directed broadcast of whatever subnet this
/// machine sits on, which is what actually gets through on a typical /24 LAN.
fn broadcast_targets(port: u16) -> Vec<SocketAddr> {
    let mut targets = vec![SocketAddr::from((Ipv4Addr::BROADCAST, port))];

    if let Some(local) = local_ipv4() {
        let o = local.octets();
        let directed = Ipv4Addr::new(o[0], o[1], o[2], 255);
        if directed != Ipv4Addr::BROADCAST {
            targets.push(SocketAddr::from((directed, port)));
        }
    }

    targets
}

/// The LAN address of this machine, found by asking the routing table which
/// interface an outbound packet would leave from. A UDP connect sends nothing,
/// so this costs no traffic and works with no internet connection.
fn local_ipv4() -> Option<Ipv4Addr> {
    let probe = UdpSocket::bind("0.0.0.0:0").ok()?;
    probe.connect("8.8.8.8:80").ok()?;
    match probe.local_addr().ok()?.ip() {
        IpAddr::V4(v4) if !v4.is_loopback() && !v4.is_unspecified() => Some(v4),
        _ => None,
    }
}
