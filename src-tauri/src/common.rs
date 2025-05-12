// src/common.rs
use socket2::{Domain, Protocol, Socket, Type};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use rcgen::{Certificate, KeyPair};
use serde::Serialize;

// LocalSend Protocol v2.1
// https://github.com/localsend/protocol/blob/main/README.md
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Message {
    pub alias: String,
    pub version: String,
    pub device_model: Option<String>,
    pub device_type: Option<String>,
    pub fingerprint: String,
    pub port: u16,
    pub protocol: String,
    pub download: Option<bool>,
    pub announce: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct PeerInfo {
    pub message: Message,
    pub remote_addrs: Vec<SocketAddr>,
}

pub fn create_udp_socket(port: u16) -> std::io::Result<Arc<tokio::net::UdpSocket>> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    socket.set_nonblocking(true)?;
    let addr = "224.0.0.48".parse().unwrap();
    socket.join_multicast_v4(&addr, &Ipv4Addr::UNSPECIFIED)?;
    #[cfg(not(windows))]
    socket.bind(&SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port).into())?;
    // https://github.com/bluejekyll/multicast-example/blob/74f8f882134305634ce5bb46a71523c0d624bd22/src/lib.rs#L40
    // On Windows, unlike all Unix variants, it is improper to bind to the multicast address
    // see https://msdn.microsoft.com/en-us/library/windows/desktop/ms737550(v=vs.85).aspx
    #[cfg(windows)]
    let addr = SocketAddr::new(Ipv4Addr::new(0, 0, 0, 0).into(), port);
    #[cfg(windows)]
    socket.bind(&socket2::SockAddr::from(addr));
    
    Ok(Arc::new(tokio::net::UdpSocket::from_std(socket.into())?))
}

pub fn generate_fingerprint_cert(cert: Certificate) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(cert.pem());
    let result = hasher.finalize();
    let fingerprint = hex::encode(result);
    fingerprint
}
pub fn generate_fingerprint_plain() -> String {
    let length: u16 = 32;
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
    let subject_alt_names = vec!["hello.world.example".to_string(),
                                 "localhost".to_string()];

    let CertifiedKey { cert, key_pair } = generate_simple_self_signed(subject_alt_names).unwrap();
    // println!("{}", cert.pem());
    println!("{}", key_pair.serialize_pem());
 
    return (cert, key_pair);
}