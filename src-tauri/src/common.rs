use rcgen::{Certificate, KeyPair};
use serde::{Deserialize, Serialize};
use socket2::{Domain, Socket, Type};
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::SystemTime;

pub(crate) const FINGERPRINT_LENGTH: u16 = 32;
pub(crate) const SESSION_LENGTH: u16 = 32;
pub(crate) const FILE_TOKEN_LENGTH: u16 = 32;
pub(crate) const FILEID_LENGTH: u16 = 32;

// LocalSend Protocol v2.1
// https://github.com/localsend/protocol/blob/main/README.md
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Message {
    pub alias: String,
    pub version: String,              // protocol version (major.minor)
    pub device_model: Option<String>, // nullable
    pub device_type: Option<String>,  // mobile | desktop | web | headless | server, nullable
    pub fingerprint: String,          // ignored in HTTPS mode
    pub port: u16,
    pub protocol: String,
    pub download: Option<bool>, // if download API (section 5.2, 5.3) is active (optional, default: false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub announce: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
enum DeviceType {
    Mobile,
    Desktop,
    Web,
    Headless,
    Server,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PrepareUploadParams {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pin: Option<String>,
}

fn empty_string_as_none<'de, D, T>(de: D) -> anyhow::Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let opt = Option::<String>::deserialize(de)?;
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => std::str::FromStr::from_str(s)
            .map_err(serde::de::Error::custom)
            .map(Some),
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PeerInfo {
    pub message: Message,
    pub remote_addrs: std::collections::VecDeque<SocketAddr>,
}
impl PeerInfo {
    pub fn add_remote_addr(&mut self, addr: SocketAddr) {
        const MAX_SIZE: usize = 6; // Set your desired limit
        if self.remote_addrs.len() == MAX_SIZE {
            self.remote_addrs.pop_front(); // Remove the oldest element
        }
        self.remote_addrs.push_back(addr);
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PrepareUploadRequestAndSessionId {
    pub sessionId: String,
    pub prepareUploadRequest: PrepareUploadRequest,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PrepareUploadRequest {
    pub info: Message,
    pub files: Files,
}

#[allow(non_camel_case_types)]
#[derive(Deserialize, Serialize, Debug, Clone)]
enum Protocol {
    http,
    https,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Files {
    // Use serde_json's custom key deserialization to handle dynamic file IDs
    #[serde(flatten)]
    pub files: std::collections::HashMap<String, UploadFile>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Sessions {
    // Use serde_json's custom key deserialization to handle dynamic file IDs
    #[serde(flatten)]
    pub sessions: HashMap<String, Session>,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Session {
    pub accepted: bool,
    pub userFeedback: bool,
    pub finished: bool,
    pub fileIdtoTokenAndUploadFile: HashMap<String, TokenAndUploadFile>,
}
#[allow(non_snake_case)]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct TokenAndUploadFile {
    pub token: String,
    pub uploadFile: UploadFile,
}
#[allow(non_snake_case)]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct UploadFile {
    pub id: String,
    pub fileName: String,
    pub size: u64, // bytes
    pub fileType: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>, // nullable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<Vec<u8>>, // nullable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>, // nullable
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Metadata {
    #[serde(
        default,
        deserialize_with = "deserialize_system_time",
        skip_serializing_if = "Option::is_none"
    )]
    pub modified: Option<std::time::SystemTime>, // nullable
    #[serde(
        default,
        deserialize_with = "deserialize_system_time",
        skip_serializing_if = "Option::is_none"
    )]
    pub accessed: Option<std::time::SystemTime>, // nullable
}
// Localsend's time is in ISO 8601 format (e.g., "2024-06-06T15:25:34.000Z").
// SystemTime does not natively support deserialization from such strings.
fn deserialize_system_time<'de, D>(
    deserializer: D,
) -> anyhow::Result<Option<std::time::SystemTime>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    if let Some(date_str) = opt {
        let parsed =
            chrono::DateTime::parse_from_rfc3339(&date_str).map_err(serde::de::Error::custom)?;
        Ok(Some(std::time::SystemTime::from(parsed)))
    } else {
        Ok(None)
    }
}

#[allow(non_snake_case)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UploadQuery {
    pub sessionId: String,
    pub fileId: String,
    pub token: String,
}

pub fn create_udp_socket(port: u16) -> std::io::Result<Arc<tokio::net::UdpSocket>> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(socket2::Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    socket.set_nonblocking(true)?;
    let addr = "224.0.0.167".parse().unwrap();

    let mut ip_addr = Ipv4Addr::UNSPECIFIED;
    // if false {
    #[cfg(target_os = "android")]
    if cfg!(target_os = "android") {
        let interfaces = pnet::datalink::interfaces();
        // for Android, if peer is connected via Soft AP (Hotspot/USB thethering),
        // may not work after joining multicast group and bind 0.0.0.0
        // so we find the most likely network range, from 192.168/16, to 172.16/12, to 10.0/8
        for interface in interfaces {
            warn!(
                "Name: {}, MAC: {:?}, IPs: {:?}, Flags: {:?}",
                interface.name, interface.mac, interface.ips, interface.flags
            );
            // prefer 192.168, 172.16.0.0 – 172.31.255.255, 10.0.0.0/8
            for ip in &interface.ips {
                if let pnet::ipnetwork::IpNetwork::V4(ipv4_network) = ip {
                    let network =
                        pnet::ipnetwork::Ipv4Network::new(Ipv4Addr::new(192, 168, 0, 0), 16)
                            .unwrap();
                    if network.contains(ipv4_network.ip()) {
                        ip_addr = ipv4_network.ip();
                        break;
                    }
                }
            }
            if ip_addr == Ipv4Addr::UNSPECIFIED {
                for ip in &interface.ips {
                    if let pnet::ipnetwork::IpNetwork::V4(ipv4_network) = ip {
                        let network =
                            pnet::ipnetwork::Ipv4Network::new(Ipv4Addr::new(172, 16, 0, 0), 12)
                                .unwrap();
                        if network.contains(ipv4_network.ip()) {
                            ip_addr = ipv4_network.ip();
                            break;
                        }
                    }
                }
            }
            if ip_addr == Ipv4Addr::UNSPECIFIED {
                for ip in &interface.ips {
                    if let pnet::ipnetwork::IpNetwork::V4(ipv4_network) = ip {
                        let network =
                            pnet::ipnetwork::Ipv4Network::new(Ipv4Addr::new(10, 0, 0, 0), 8)
                                .unwrap();
                        if network.contains(ipv4_network.ip()) {
                            ip_addr = ipv4_network.ip();
                            break;
                        }
                    }
                }
            }
        }
    }
    info!("Using IP address: {}", ip_addr);
    socket.join_multicast_v4(&addr, &ip_addr)?;
    socket.bind(&SocketAddrV4::new(ip_addr, port).into())?;
    // may not be able to send udp message on Android:
    // W localshare_lib::localsend: [localshare_lib::localsend] Failed to send multicast message: Operation not permitted (os error 1)
    // this may also fail when no NIC is active:
    // called `Result::unwrap()` on an `Err` value: Os { code: 19, kind: Uncategorized, message: "No such device" }
    Ok(Arc::new(tokio::net::UdpSocket::from_std(socket.into())?))
    // Ok(Arc::new(socket.into()))
}

pub fn generate_fingerprint_cert(cert: Certificate) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(cert.pem());
    let result = hasher.finalize();
    let fingerprint = hex::encode(result);
    fingerprint
}
pub fn generate_random_string(length: u16) -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    let mut fingerprint = String::new();
    for _ in 0..length {
        let byte = rng.random_range(0..=255);
        fingerprint.push_str(&format!("{:02x}", byte));
    }
    fingerprint
}
pub fn generate_cert_key() -> (Certificate, KeyPair) {
    use rcgen::{generate_simple_self_signed, CertifiedKey};
    // Generate a certificate that's valid for "localhost" and "hello.world.example"
    let subject_alt_names = vec!["hello.world.example".to_string(), "localhost".to_string()];

    let CertifiedKey { cert, key_pair } = generate_simple_self_signed(subject_alt_names).unwrap();
    // info!("{}", cert.pem());
    info!("{}", key_pair.serialize_pem());

    return (cert, key_pair);
}

// Chat message structures
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChatMessage {
    pub id: String,
    pub sender_fingerprint: String,
    pub sender_alias: String,
    pub receiver_fingerprint: String,
    pub content: String,
    pub timestamp: SystemTime,
    pub read: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ChatHistory {
    pub messages: Vec<ChatMessage>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChatSession {
    pub peer_fingerprint: String,
    pub peer_alias: String,
    pub last_message: Option<ChatMessage>,
    pub unread_count: u32,
    pub color: String, // Background color for this chat
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ChatSessions {
    pub sessions: HashMap<String, ChatSession>, // Key is peer_fingerprint
}

// Generate a random color for chat backgrounds
pub fn generate_random_color() -> String {
    use rand::Rng;
    let mut rng = rand::rng();

    // Generate pastel colors (lighter shades) for better readability
    let r = rng.random_range(180..=240);
    let g = rng.random_range(180..=240);
    let b = rng.random_range(180..=240);

    format!("#{:02x}{:02x}{:02x}", r, g, b)
}
