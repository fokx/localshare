use dotenvy_macro::dotenv;
use rustls::RootCertStore;
use rustls_pemfile::Item;
use rustls_pki_types::CertificateDer;
use std::{
    fs::{self, File},
    io::BufReader,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    str::FromStr,
};
use tokio::net;

use crate::tuicc::error::Error;

pub fn load_certs(paths: Vec<PathBuf>, disable_native: bool) -> Result<RootCertStore, Error> {
    let mut certs = RootCertStore::empty();

    let client_cert = dotenv!("CLIENT_CERT");
    if !client_cert.is_empty() {
        let mut reader = std::io::BufReader::new(client_cert.as_bytes());
        while let Ok(Some(item)) = rustls_pemfile::read_one(&mut reader) {
            if let Item::X509Certificate(cert) = item {
                certs.add(CertificateDer::from(cert))?;
            }
        }
        if certs.is_empty() {
            certs.add(CertificateDer::from(client_cert.as_bytes().to_vec()))?;
        }
    } else {
        for path in &paths {
            let mut file = BufReader::new(File::open(path)?);

            while let Ok(Some(item)) = rustls_pemfile::read_one(&mut file) {
                if let Item::X509Certificate(cert) = item {
                    certs.add(CertificateDer::from(cert))?;
                }
            }
        }
        if certs.is_empty() {
            for path in &paths {
                certs.add(CertificateDer::from(fs::read(path)?))?;
            }
        }
    }

    if !disable_native {
        for cert in rustls_native_certs::load_native_certs().expect("could not load native certs") {
            certs.add(cert).unwrap();
        }
    }

    Ok(certs)
}

pub struct ServerAddr {
    domain: String,
    port: u16,
    ip: Option<IpAddr>,
}

impl ServerAddr {
    pub fn new(domain: String, port: u16, ip: Option<IpAddr>) -> Self {
        Self { domain, port, ip }
    }

    pub fn server_name(&self) -> &str {
        &self.domain
    }

    pub async fn resolve(&self) -> Result<impl Iterator<Item = SocketAddr>, Error> {
        if let Some(ip) = self.ip {
            Ok(vec![SocketAddr::from((ip, self.port))].into_iter())
        } else {
            Ok(net::lookup_host((self.domain.as_str(), self.port))
                .await?
                .collect::<Vec<_>>()
                .into_iter())
        }
    }
}

#[derive(Clone, Copy)]
pub enum UdpRelayMode {
    Native,
    Quic,
}

impl FromStr for UdpRelayMode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("native") {
            Ok(Self::Native)
        } else if s.eq_ignore_ascii_case("quic") {
            Ok(Self::Quic)
        } else {
            Err("invalid UDP relay mode")
        }
    }
}

pub enum CongestionControl {
    Cubic,
    NewReno,
    Bbr,
}

impl FromStr for CongestionControl {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("cubic") {
            Ok(Self::Cubic)
        } else if s.eq_ignore_ascii_case("new_reno") || s.eq_ignore_ascii_case("newreno") {
            Ok(Self::NewReno)
        } else if s.eq_ignore_ascii_case("bbr") {
            Ok(Self::Bbr)
        } else {
            Err("invalid congestion control")
        }
    }
}
