use dotenvy_macro::dotenv;
use socks5_proto::{Address, Reply};
use socks5_server::{
    connection::{associate, bind, connect},
    Associate, Bind, Connect,
};
use tokio::io::{self, AsyncWriteExt};
use tokio_util::compat::FuturesAsyncReadCompatExt;

use tuic::Address as TuicAddress;

use crate::tuicc::connection::{Connection as TuicConnection, ERROR_CODE};

use super::{udp_session::UdpSession, Server, UDP_SESSIONS};

impl Server {
    pub async fn handle_associate(
        assoc: Associate<associate::state::NeedReply>,
        assoc_id: u16,
        dual_stack: Option<bool>,
        max_pkt_size: usize,
    ) {
        let peer_addr = assoc.peer_addr().unwrap();
        let local_ip = assoc.local_addr().unwrap().ip();

        match UdpSession::new(assoc_id, peer_addr, local_ip, dual_stack, max_pkt_size) {
            Ok(session) => {
                let local_addr = session.local_addr().unwrap();
                log::debug!(
                    "[socks5] [{peer_addr}] [associate] [{assoc_id:#06x}] bound to {local_addr}"
                );

                let mut assoc = match assoc
                    .reply(Reply::Succeeded, Address::SocketAddress(local_addr))
                    .await
                {
                    Ok(assoc) => assoc,
                    Err((err, mut conn)) => {
                        log::warn!("[socks5] [{peer_addr}] [associate] [{assoc_id:#06x}] command reply error: {err}");
                        let _ = conn.shutdown().await;
                        return;
                    }
                };

                UDP_SESSIONS
                    .get()
                    .unwrap()
                    .lock()
                    .insert(assoc_id, session.clone());

                let handle_local_incoming_pkt = async move {
                    loop {
                        let (pkt, target_addr) = match session.recv().await {
                            Ok(res) => res,
                            Err(err) => {
                                log::warn!("[socks5] [{peer_addr}] [associate] [{assoc_id:#06x}] failed to receive UDP packet: {err}");
                                continue;
                            }
                        };

                        let forward = async move {
                            let target_addr = match target_addr {
                                Address::DomainAddress(domain, port) => {
                                    let domain = String::from_utf8_lossy(&domain);
                                    TuicAddress::DomainAddress(domain.as_ref().to_string(), port)
                                }
                                Address::SocketAddress(addr) => TuicAddress::SocketAddress(addr),
                            };

                            match TuicConnection::get().await {
                                Ok(conn) => conn.packet(pkt, target_addr, assoc_id).await,
                                Err(err) => Err(err),
                            }
                        };

                        tokio::spawn(async move {
                            match forward.await {
                                Ok(()) => {}
                                Err(err) => {
                                    log::warn!("[socks5] [{peer_addr}] [associate] [{assoc_id:#06x}] failed relaying UDP packet: {err}");
                                }
                            }
                        });
                    }
                };

                match tokio::select! {
                    res = assoc.wait_close() => res,
                    _ = handle_local_incoming_pkt => unreachable!(),
                } {
                    Ok(()) => {}
                    Err(err) => {
                        log::warn!("[socks5] [{peer_addr}] [associate] [{assoc_id:#06x}] associate connection error: {err}")
                    }
                }

                log::debug!(
                    "[socks5] [{peer_addr}] [associate] [{assoc_id:#06x}] stopped associating"
                );

                UDP_SESSIONS
                    .get()
                    .unwrap()
                    .lock()
                    .remove(&assoc_id)
                    .unwrap();

                let res = match TuicConnection::get().await {
                    Ok(conn) => conn.dissociate(assoc_id).await,
                    Err(err) => Err(err),
                };

                match res {
                    Ok(()) => {}
                    Err(err) => log::warn!("[socks5] [{peer_addr}] [associate] [{assoc_id:#06x}] failed stopping UDP relaying session: {err}"),
                }
            }
            Err(err) => {
                log::warn!("[socks5] [{peer_addr}] [associate] [{assoc_id:#06x}] failed setting up UDP associate session: {err}");

                match assoc
                    .reply(Reply::GeneralFailure, Address::unspecified())
                    .await
                {
                    Ok(mut assoc) => {
                        let _ = assoc.close().await;
                    }
                    Err((err, mut conn)) => {
                        let _ = conn.shutdown().await;
                        log::warn!("[socks5] [{peer_addr}] [associate] [{assoc_id:#06x}] command reply error: {err}")
                    }
                }
            }
        }
    }

    pub async fn handle_bind(bind: Bind<bind::state::NeedFirstReply>) {
        let peer_addr = bind.peer_addr().unwrap();
        log::warn!("[socks5] [{peer_addr}] [bind] command not supported");

        match bind
            .reply(Reply::CommandNotSupported, Address::unspecified())
            .await
        {
            Ok(mut bind) => {
                let _ = bind.close().await;
            }
            Err((err, mut conn)) => {
                let _ = conn.shutdown().await;
                log::warn!("[socks5] [{peer_addr}] [bind] command reply error: {err}");
            }
        }
    }

    pub async fn handle_connect(conn: Connect<connect::state::NeedReply>, addr: Address) {
        let peer_addr = conn.peer_addr().unwrap();
        let target_addr = match addr {
            Address::DomainAddress(domain, port) => TuicAddress::DomainAddress(
                String::from_utf8_lossy(&domain).as_ref().to_string(),
                port,
            ),
            Address::SocketAddress(addr) => TuicAddress::SocketAddress(addr),
        };
        let whitelist_domains = dotenv!("WHITELISTED_DOMAINS");
        let mut target_in_whitelist = false;

        if whitelist_domains.is_empty() {
            target_in_whitelist = true;
        } else {
            /* this stealthy tuic client should only proxy requests to target
            the server should also ensure the client cannot access websites other than target,
            to avoid malicious use of the server.
            */
            let white_listed_ports: Vec<&str> = dotenv!("WHITELISTED_PORTS").split(",").collect();
            if let TuicAddress::DomainAddress(domain, port) = &target_addr {
                for whitelisted_domain in dotenv!("WHITELISTED_DOMAINS").split(",") {
                    if domain == whitelisted_domain
                        || domain.ends_with(&format!(".{}", whitelisted_domain))
                    {
                        if white_listed_ports.contains(&&*port.to_string()) {
                            target_in_whitelist = true;
                            break;
                        }
                    }
                }
            }
        }

        if target_in_whitelist {
            let relay = match TuicConnection::get().await {
                Ok(conn) => conn.connect(target_addr.clone()).await,
                Err(err) => Err(err),
            };

            match relay {
                Ok(relay) => {
                    let mut relay = relay.compat();

                    match conn.reply(Reply::Succeeded, Address::unspecified()).await {
                        Ok(mut conn) => match io::copy_bidirectional(&mut conn, &mut relay).await {
                            Ok(_) => {}
                            Err(err) => {
                                let _ = conn.shutdown().await;
                                let _ = relay.get_mut().reset(ERROR_CODE);
                                log::warn!("[socks5] [{peer_addr}] [connect] [{target_addr}] TCP stream relaying error: {err}");
                            }
                        },
                        Err((err, mut conn)) => {
                            let _ = conn.shutdown().await;
                            let _ = relay.shutdown().await;
                            log::warn!("[socks5] [{peer_addr}] [connect] [{target_addr}] command reply error: {err}");
                        }
                    }
                }
                Err(err) => {
                    log::warn!("[socks5] [{peer_addr}] [connect] [{target_addr}] unable to relay TCP stream: {err}");

                    match conn
                        .reply(Reply::GeneralFailure, Address::unspecified())
                        .await
                    {
                        Ok(mut conn) => {
                            let _ = conn.shutdown().await;
                        }
                        Err((err, mut conn)) => {
                            let _ = conn.shutdown().await;
                            log::warn!("[socks5] [{peer_addr}] [connect] [{target_addr}] command reply error: {err}")
                        }
                    }
                }
            }
        } else {
            let target_addr_str = format!("{}", target_addr);
            // create a stream directly to target_addr
            let relay = tokio::net::TcpStream::connect(target_addr_str).await;
            if let Err(err) = relay {
                log::warn!("[socks5bypass] [{peer_addr}] [connect] [{target_addr}] failed to connect to target: {err}");
                match conn
                    .reply(Reply::HostUnreachable, Address::unspecified())
                    .await
                {
                    Ok(mut conn) => {
                        let _ = conn.shutdown().await;
                    }
                    Err((err, mut conn)) => {
                        let _ = conn.shutdown().await;
                        log::warn!("[socks5bypass] [{peer_addr}] [connect] [{target_addr}] command reply error: {err}");
                    }
                }
                return;
            } else {
                let mut relay = relay.unwrap();
                log::warn!(
                    "[socks5bypass] [{peer_addr}] [connect] [{target_addr}] connected to target"
                );
                match conn.reply(Reply::Succeeded, Address::unspecified()).await {
                    Ok(mut conn) => match io::copy_bidirectional(&mut conn, &mut relay).await {
                        Ok(_) => {}
                        Err(err) => {
                            let _ = conn.shutdown().await;
                            let _ = relay.shutdown().await;
                            log::warn!("[socks5bypass] [{peer_addr}] [connect] [{target_addr}] TCP stream relaying error: {err}");
                        }
                    },
                    Err((err, mut conn)) => {
                        let _ = conn.shutdown().await;
                        let _ = relay.shutdown().await;
                        log::warn!("[socks5bypass] [{peer_addr}] [connect] [{target_addr}] command reply error: {err}");
                    }
                }
            }
        }
    }
}
