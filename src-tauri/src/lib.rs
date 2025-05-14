#[macro_use]
extern crate log;
use futures::StreamExt;
use log::{debug, error, info, trace, warn};
use network_interface::NetworkInterface;
use network_interface::NetworkInterfaceConfig;
use sysinfo::{Disks, System};
use tauri::path::PathResolver;
use tauri::{Emitter, Listener, Manager};
use tauri_plugin_android_fs::{
    AndroidFs, AndroidFsExt, FileUri, InitialLocation, PersistableAccessMode, PrivateDir,
};

// use tokio::task::JoinHandle;
// use tauri::async_runtime::TokioJoinHandle;
use tauri_plugin_log::{Target, TargetKind};
use tauri_plugin_store::{JsonValue, StoreExt};
use tokio;
use tokio::sync::oneshot;
mod args;
mod auth;
mod http_logger;
mod http_utils;
mod logger;
mod server;
mod utils;

use crate::args::{build_cli, print_completions, Args};
use crate::server::Server;
#[cfg(feature = "tls")]
use crate::utils::{load_certs, load_private_key};

use anyhow::{anyhow, Context, Result};
use args::BindAddr;
use clap_complete::Shell;
use futures_util::future::join_all;

use hyper::{body::Incoming, service::service_fn};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder,
};
use std::net::{IpAddr, TcpListener as StdTcpListener};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;
use tokio::time::timeout;
use tokio::{net::TcpListener, task::JoinHandle};
#[cfg(feature = "tls")]
use tokio_rustls::{rustls::ServerConfig, TlsAcceptor};

use crate::auth::AccessControl;
use axum::{
    extract::{Query, Request, State},
    handler::HandlerWithoutStateExt,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use std::net::SocketAddr;
use tower::ServiceExt;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
const FINGERPRINT_LENGTH: u16 = 32;
const SESSION_LENGTH: u16 = 32;
mod common;

use crate::common::generate_random_string;
use axum::extract::Path;
use common::{create_udp_socket, Message, PeerInfo};
// use std::io::prelude::*;
use futures::{Stream, TryStreamExt};
use std::collections::HashMap;
use std::io::{self, Error, Write};
use serde::Deserialize;
use serde_json::Value;
use tokio::{fs::File, io::BufWriter};
use tokio_util::io::StreamReader;
async fn handler_register(
    my_response: Arc<Message>,
    Json(payload): Json<Message>,
) -> () {
// ) -> Json<Message> {
    // Here you can process the payload as needed
    debug!("axum register_handler received message: {:?}", payload);
    // TODO: register peer

    // Use my_response instead of creating a new response Message
    // Return the pre-defined response as JSON
    // Json((*my_response).clone())
    ()
}
#[derive(serde::Deserialize, Debug, Clone)]
struct PrepareUploadParams {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pin: Option<String>,
}

fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
        D: serde::Deserializer<'de>,
        T: std::str::FromStr,
        T::Err: std::fmt::Display,
{
    let opt = Option::<String>::deserialize(de)?;
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => std::str::FromStr::from_str(s).map_err(serde::de::Error::custom).map(Some),
    }
}

async fn handler_prepare_upload(
    State(app_handle): State<tauri::AppHandle>,
    Query(params): Query<PrepareUploadParams>,
    Json(payload): Json<PrepareUploadRequest>,
) -> Json<HashMap<String, JsonValue>> {
    debug!("axum handler_prepare_upload Payload: {:?}", payload);
    debug!("axum handler_prepare_upload Received request with params: {:?}", params);
    // waiting the state of whether user has accepted the request for 10s, if not, return error
    let sessionId = generate_random_string(SESSION_LENGTH);
    app_handle.emit("prepare-upload", PrepareUploadRequestAndSessionId{sessionId: sessionId.clone(), prepareUploadRequest: payload.clone() }).unwrap();

    let sessions_state = app_handle.state::<Mutex<Sessions>>();
    let mut sessions = sessions_state.lock().unwrap();

    let mut files_tokens = HashMap::new();

    for (fileId, _) in &payload.files.files {
        let token = format!("token_for_{}", fileId); // Replace it with actual token generation logic
        files_tokens.insert(fileId.clone(), token);
    }

    sessions.sessions.insert(sessionId.clone(), files_tokens.clone());

    Json({
        let mut response = HashMap::new();
        response.insert("sessionId".to_string(), serde_json::to_value(sessionId.clone()).unwrap());
        response.insert(
            "files".to_string(),
            serde_json::to_value(files_tokens).unwrap(),
        );
        // response.insert(
        //     "files".to_string(),
        //     serde_json::to_value(files_tokens).unwrap().to_string(),
        // );
        response
    })
}

async fn handler_upload(
    Query(query_params): Query<UploadQuery>,
    body: axum::body::Body,
) -> Json<Result<(), String>> {
    debug!("axum handler_prepare_upload query_params: {:?}", query_params);

    // Verify the sessionId, fileId, and token for security
    // if query_params.sessionId != "mySessionId"
    //     || query_params.fileId != "fileId"
    //     || query_params.token != "someFileToken"
    // {
    //     return Json(Err("Invalid session, fileId or token".to_string()));
    // }
    let res = async {
        println!("{:?}", query_params);
        let path = format!("/tmp/{:?}", query_params.fileId);
        // Save binary data to the file
        let body_with_io_error = body.into_data_stream().map_err(io::Error::other);
        let body_reader = StreamReader::new(body_with_io_error);
        futures::pin_mut!(body_reader);
        // Create the file. `File` implements `AsyncWrite`.
        let mut file = BufWriter::new(File::create(path).await.unwrap());
        tokio::io::copy(&mut body_reader, &mut file).await.unwrap();
        Ok::<_, io::Error>(())
    }
    .await;
    match res {
        Ok(_) => Json(Ok(())),
        Err(e) => {
            error!("Error saving file: {:?}", e);
            Json(Err(String::from(format!("Error saving file: {:?}", e))))
        },
    }
}

async fn periodic_announce(my_response: Arc<Message>) -> std::io::Result<()> {
    let port = 53317;
    let udp = create_udp_socket(port)?;
    let addr: std::net::Ipv4Addr = "224.0.0.167".parse().unwrap();
    let mut count = 0;
    let ANNOUNCE_INTERVAL = 3600;
    loop {
        debug!("announce sequence {}", count);
        let my_response_new = Message {
            alias: my_response.alias.clone(),
            version: my_response.version.clone(),
            device_model: my_response.device_model.clone(),
            device_type: my_response.device_type.clone(),
            fingerprint: my_response.fingerprint.clone(),
            port: my_response.port,
            protocol: my_response.protocol.clone(),
            download: my_response.download,
            announce: Some(true),
        };
        udp.send_to(
            &serde_json::to_vec(&my_response_new).expect("Failed to serialize Message"),
            (addr, port),
        )
        .await
        .expect("cannot send message to socket");
        tokio::time::sleep(std::time::Duration::from_secs(ANNOUNCE_INTERVAL)).await;
        count += 1;
        break
    }
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct PrepareUploadRequestAndSessionId {
    sessionId: String,
    prepareUploadRequest: PrepareUploadRequest,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct PrepareUploadRequest {
    info: Info,
    files: Files,
}

#[derive(serde::Deserialize, serde::Serialize, Debug,Clone)]
struct Info {
    alias: String,
    version: String,                 // protocol version (major.minor)
    device_model: Option<String>,    // nullable
    device_type: Option<DeviceType>, // mobile | desktop | web | headless | server, nullable
    fingerprint: String,             // ignored in HTTPS mode
    port: u16,
    protocol: Protocol,
    download: bool, // if download API (section 5.2, 5.3) is active (optional, default: false)
}

#[derive(serde::Deserialize, serde::Serialize, Debug,Clone)]
enum DeviceType {
    Mobile,
    Desktop,
    Web,
    Headless,
    Server,
}

#[derive(serde::Deserialize, serde::Serialize, Debug,Clone)]
enum Protocol {
    http,
    https,
}

#[derive(serde::Deserialize, serde::Serialize, Debug,Clone)]
struct Files {
    // Use serde_json's custom key deserialization to handle dynamic file IDs
    #[serde(flatten)]
    files: std::collections::HashMap<String, UploadFile>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug,Clone, Default)]
struct Sessions {
    // Use serde_json's custom key deserialization to handle dynamic file IDs
    #[serde(flatten)]
    sessions: std::collections::HashMap<String, HashMap<String, String>>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug,Clone)]
struct UploadFile {
    id: String,
    fileName: String,
    size: u64, // bytes
    fileType: String,
    sha256: Option<String>,   // nullable
    preview: Option<Vec<u8>>, // nullable
    metadata: Option<Metadata>,
}


#[derive(serde::Deserialize, serde::Serialize, Debug,Clone)]
struct Metadata {
    #[serde(default, deserialize_with = "deserialize_system_time")]
    modified: Option<std::time::SystemTime>,
    accessed: Option<std::time::SystemTime>,
}
// Localsend's time is in ISO 8601 format (e.g., "2024-06-06T15:25:34.000Z").
// SystemTime does not natively support deserialization from such strings.
fn deserialize_system_time<'de, D>(deserializer: D) -> Result<Option<std::time::SystemTime>, D::Error>
where
        D: serde::Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    if let Some(date_str) = opt {
        let parsed = chrono::DateTime::parse_from_rfc3339(&date_str)
                .map_err(serde::de::Error::custom)?;
        Ok(Some(std::time::SystemTime::from(parsed)))
    } else {
        Ok(None)
    }
}
#[derive(serde::Deserialize, serde::Serialize, Debug,Clone)]
struct UploadQuery {
    sessionId: String,
    fileId: String,
    token: String,
}

#[tokio::test]
async fn client_test() -> std::io::Result<()> {
    // cargo test -- --nocapture
    // https://stackoverflow.com/questions/25106554/why-doesnt-println-work-in-rust-unit-tests
    let my_fingerprint = generate_random_string(FINGERPRINT_LENGTH);
    debug!("test client fingerprint : {}", my_fingerprint);
    let port = 53317;
    let my_response = Arc::new(Message {
        alias: my_fingerprint[0..6].to_string(),
        version: "2.1".to_string(),
        device_model: Some("unimplemented".to_string()),
        device_type: Some("unimplemented".to_string()),
        fingerprint: my_fingerprint.clone(),
        port,
        protocol: "http".to_string(),
        download: Some(true),
        announce: Some(false),
    });

    let my_response_for_announce = Arc::clone(&my_response);
    let my_response_clone = Arc::clone(&my_response);

    periodic_announce(my_response_for_announce).await?;
    // POST to "/api/localsend/v2/register"
    let client = reqwest::Client::new();
    let res = client
        .post(format!("http://127.0.0.1:53317/api/localsend/v2/register"))
        .json(&*my_response_clone)
        .send()
        .await;
    match res {
        Ok(response) => {
            debug!("Response: {:?}", response);
        }
        Err(e) => {
            debug!("Error: {:?}", e);
        }
    }
    Ok(())
}
#[tauri::command(rename_all = "snake_case")]
async fn announce_once(my_response: tauri::State<'_, Message>) -> Result<String, String> {
        let port = 53317;
        let udp = create_udp_socket(port).unwrap();
        let addr: std::net::Ipv4Addr = "224.0.0.167".parse().unwrap();
        let mut count = 0;
        let ANNOUNCE_INTERVAL = 3600;
        loop {
            debug!("announce sequence {}", count);
            let my_response_new = Message {
                alias: my_response.alias.clone(),
                version: my_response.version.clone(),
                device_model: my_response.device_model.clone(),
                device_type: my_response.device_type.clone(),
                fingerprint: my_response.fingerprint.clone(),
                port: my_response.port,
                protocol: my_response.protocol.clone(),
                download: my_response.download,
                announce: Some(true),
            };
            udp.send_to(
                &serde_json::to_vec(&my_response_new).expect("Failed to serialize Message"),
                (addr, port),
            )
                    .await
                    .expect("cannot send message to socket");
            tokio::time::sleep(std::time::Duration::from_secs(ANNOUNCE_INTERVAL)).await;
            count += 1;
            break
        }
    return Ok("started".to_string());
}
#[tauri::command(rename_all = "snake_case")]
fn toggle_server(
    app: tauri::AppHandle,
    server_handle: tauri::State<Arc<Mutex<Option<oneshot::Sender<()>>>>>,
    server_port: usize,
    serve_path: String,
    require_auth: bool,
    auth_user: String,
    auth_passwd: String,
    allow_upload: bool,
) -> Result<String, String> {
    debug!("using server_port: {}", server_port);
    let mut state_locked = server_handle.lock().unwrap();

    if let Some(shutdown_tx) = state_locked.take() {
        // Stop the server
        warn!("Stopping server");
        let _ = shutdown_tx.send(()); // Send the shutdown signal
        return Ok("stopped".to_string());
    } else {
        // Start the server
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        // let runtime = tokio::runtime::Runtime::new().unwrap();
        let join_handle = tauri::async_runtime::spawn(dufs_main(
            shutdown_rx,
            server_port,
            serve_path,
            require_auth,
            auth_user,
            auth_passwd,
            allow_upload,
        ));
        // runtime.spawn(async move {
        //     actix_main(shutdown_rx).await;
        // });
        *state_locked = Some(shutdown_tx);
        return Ok("started".to_string());
    }
}

async fn dufs_main(
    shutdown_rx: oneshot::Receiver<()>,
    server_port: usize,
    serve_path: String,
    require_auth: bool,
    auth_user: String,
    auth_passwd: String,
    allow_upload: bool,
) -> Result<()> {
    let cmd = build_cli();
    let matches = cmd.get_matches();
    if let Some(generator) = matches.get_one::<Shell>("completions") {
        let mut cmd = build_cli();
        print_completions(*generator, &mut cmd);
        return Ok(());
    }
    let mut args = Args::parse(matches).unwrap();
    if require_auth {
        let rules = vec![format!("{}:{}@/:rw", auth_user, auth_passwd)];
        let rules: Vec<_> = rules.iter().map(|s| s.as_str()).collect();
        args.auth = AccessControl::new(&rules).unwrap();
    }
    args.serve_path = serve_path.parse()?;
    args.allow_upload = allow_upload;
    args.allow_search = true;
    args.allow_archive = true;
    // logger::init(args.log_file.clone()).map_err(|e| anyhow!("Failed to init logger, {e}"))?;
    // let (new_addrs, print_addrs) = check_addrs(&args)?;
    // args.addrs = new_addrs;
    let running = Arc::new(AtomicBool::new(true));
    // let listening = print_listening(&args, &print_addrs)?;
    // let listener = create_listener(SocketAddr::new("0.0.0.0".parse()?, 4804))
    //         .with_context(|| format!("Failed to bind"))?;
    let listener = TcpListener::bind(format!("0.0.0.0:{}", server_port)).await?;
    let http = hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new());
    // let mut http = http1::Builder::new();
    let graceful = hyper_util::server::graceful::GracefulShutdown::new();
    let mut signal = std::pin::pin!(shutdown_rx);
    let server_handle = Arc::new(Server::init(args, running.clone())?);
    loop {
        tokio::select! {
            conn = listener.accept() => {
                let (stream, peer_addr) = match conn {
                    Ok(conn) => conn,
                    Err(e) => {
                        warn!("accept error: {}", e);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                };
                debug!("incomming connection accepted: {}", peer_addr);
                // let io = TokioIo::new(stream);
                let io = hyper_util::rt::TokioIo::new(Box::pin(stream));
                let srever_handle_clone = server_handle.clone();
                // let conn = http.serve_connection(io, service_fn(hello));
                let conn =
                http.serve_connection_with_upgrades(io, hyper::service::service_fn( move |request: Request<Incoming>|
                    srever_handle_clone.clone().call(request, Some(peer_addr))
                ));
                // watch this connection
                let fut = graceful.watch(conn.into_owned());
                tokio::spawn(async move {
                    if let Err(e) = fut.await {
                        warn!("Error serving connection: {:?}", e);
                    }
                    debug!("connection dropped: {}", peer_addr);
                });
            },
            _ = signal.as_mut() => {
                drop(listener);
                warn!("graceful shutdown signal received");
                // stop the accept loop
                break;
            }
            // _ = async {
            //         handle.await.unwrap();
            //         // Help the rust type inferencer out
            //         Ok::<_,std::io::Error>(())
            //     } => {}
            // _ = rx => {
            //     debug!("terminating async task");
            //     running.store(false, Ordering::SeqCst);
            //     debug!("async task terminated");
            // },
        }
    }

    tokio::select! {
        _ = graceful.shutdown() => {
            warn!("all connections gracefully closed");
        },
        _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
            warn!("timed out wait for all connections to close");
        }
    }

    Ok(())
}

fn serve(args: Args, running: Arc<AtomicBool>) -> Result<Vec<JoinHandle<()>>> {
    let addrs = args.addrs.clone();
    let port = args.port;
    let tls_config = (args.tls_cert.clone(), args.tls_key.clone());
    let server_handle = Arc::new(Server::init(args, running)?);
    let mut handles = vec![];
    for bind_addr in addrs.iter() {
        let server_handle = server_handle.clone();
        match bind_addr {
            BindAddr::IpAddr(ip) => {
                let listener = create_listener(SocketAddr::new(*ip, port))
                    .with_context(|| format!("Failed to bind `{ip}:{port}`"))?;

                match &tls_config {
                    #[cfg(feature = "tls")]
                    (Some(cert_file), Some(key_file)) => {
                        let certs = load_certs(cert_file)?;
                        let key = load_private_key(key_file)?;
                        let mut config = ServerConfig::builder()
                            .with_no_client_auth()
                            .with_single_cert(certs, key)?;
                        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
                        let config = Arc::new(config);
                        let tls_accepter = TlsAcceptor::from(config);
                        let handshake_timeout = Duration::from_secs(10);

                        let handle = tokio::spawn(async move {
                            loop {
                                let Ok((stream, addr)) = listener.accept().await else {
                                    continue;
                                };
                                let Some(stream) =
                                    timeout(handshake_timeout, tls_accepter.accept(stream))
                                        .await
                                        .ok()
                                        .and_then(|v| v.ok())
                                else {
                                    continue;
                                };
                                let stream = TokioIo::new(stream);
                                tokio::spawn(handle_stream(
                                    server_handle.clone(),
                                    stream,
                                    Some(addr),
                                ));
                            }
                        });

                        handles.push(handle);
                    }
                    (None, None) => {
                        let handle = tokio::spawn(async move {
                            loop {
                                let Ok((stream, addr)) = listener.accept().await else {
                                    continue;
                                };
                                let stream = TokioIo::new(stream);
                                tokio::spawn(handle_stream(
                                    server_handle.clone(),
                                    stream,
                                    Some(addr),
                                ));
                            }
                        });
                        handles.push(handle);
                    }
                    _ => {
                        unreachable!()
                    }
                };
            }
            #[cfg(unix)]
            BindAddr::SocketPath(path) => {
                let socket_path = if path.starts_with("@")
                    && cfg!(any(target_os = "linux", target_os = "android"))
                {
                    let mut path_buf = path.as_bytes().to_vec();
                    path_buf[0] = b'\0';
                    unsafe { std::ffi::OsStr::from_encoded_bytes_unchecked(&path_buf) }
                        .to_os_string()
                } else {
                    let _ = std::fs::remove_file(path);
                    path.into()
                };
                let listener = tokio::net::UnixListener::bind(socket_path)
                    .with_context(|| format!("Failed to bind `{}`", path))?;
                let handle = tokio::spawn(async move {
                    loop {
                        let Ok((stream, _addr)) = listener.accept().await else {
                            continue;
                        };
                        let stream = TokioIo::new(stream);
                        tokio::spawn(handle_stream(server_handle.clone(), stream, None));
                    }
                });

                handles.push(handle);
            }
        }
    }
    Ok(handles)
}

async fn handle_stream<T>(handle: Arc<Server>, stream: TokioIo<T>, addr: Option<SocketAddr>)
where
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let hyper_service =
        service_fn(move |request: Request<Incoming>| handle.clone().call(request, addr));

    match Builder::new(TokioExecutor::new())
        .serve_connection_with_upgrades(stream, hyper_service)
        .await
    {
        Ok(()) => {}
        Err(_err) => {
            // This error only appears when the client doesn't send a request and terminate the connection.
            //
            // If client sends one request then terminate connection whenever, it doesn't appear.
        }
    }
}

fn create_listener(addr: SocketAddr) -> Result<TcpListener> {
    use socket2::{Domain, Protocol, Socket, Type};
    let socket = Socket::new(Domain::for_address(addr), Type::STREAM, Some(Protocol::TCP))?;
    if addr.is_ipv6() {
        socket.set_only_v6(true)?;
    }
    socket.set_reuse_address(true)?;
    socket.bind(&addr.into())?;
    socket.listen(1024 /* Default backlog */)?;
    let std_listener = StdTcpListener::from(socket);
    std_listener.set_nonblocking(true)?;
    let listener = TcpListener::from_std(std_listener)?;
    Ok(listener)
}

fn check_addrs(args: &Args) -> Result<(Vec<BindAddr>, Vec<BindAddr>)> {
    let mut new_addrs = vec![];
    let mut print_addrs = vec![];
    let (ipv4_addrs, ipv6_addrs) = interface_addrs()?;
    for bind_addr in args.addrs.iter() {
        match bind_addr {
            BindAddr::IpAddr(ip) => match &ip {
                IpAddr::V4(_) => {
                    if !ipv4_addrs.is_empty() {
                        new_addrs.push(bind_addr.clone());
                        if ip.is_unspecified() {
                            print_addrs.extend(ipv4_addrs.clone());
                        } else {
                            print_addrs.push(bind_addr.clone());
                        }
                    }
                }
                IpAddr::V6(_) => {
                    if !ipv6_addrs.is_empty() {
                        new_addrs.push(bind_addr.clone());
                        if ip.is_unspecified() {
                            print_addrs.extend(ipv6_addrs.clone());
                        } else {
                            print_addrs.push(bind_addr.clone())
                        }
                    }
                }
            },
            #[cfg(unix)]
            _ => {
                new_addrs.push(bind_addr.clone());
                print_addrs.push(bind_addr.clone())
            }
        }
    }
    print_addrs.sort_unstable();
    Ok((new_addrs, print_addrs))
}

fn interface_addrs() -> Result<(Vec<BindAddr>, Vec<BindAddr>)> {
    let (mut ipv4_addrs, mut ipv6_addrs) = (vec![], vec![]);
    let ifaces =
        if_addrs::get_if_addrs().with_context(|| "Failed to get local interface addresses")?;
    for iface in ifaces.into_iter() {
        let ip = iface.ip();
        if ip.is_ipv4() {
            ipv4_addrs.push(BindAddr::IpAddr(ip))
        }
        if ip.is_ipv6() {
            ipv6_addrs.push(BindAddr::IpAddr(ip))
        }
    }
    Ok((ipv4_addrs, ipv6_addrs))
}

fn print_listening(args: &Args, print_addrs: &[BindAddr]) -> Result<String> {
    let mut output = String::new();
    let urls = print_addrs
        .iter()
        .map(|bind_addr| match bind_addr {
            BindAddr::IpAddr(addr) => {
                let addr = match addr {
                    IpAddr::V4(_) => format!("{}:{}", addr, args.port),
                    IpAddr::V6(_) => format!("[{}]:{}", addr, args.port),
                };
                let protocol = if args.tls_cert.is_some() {
                    "https"
                } else {
                    "http"
                };
                format!("{}://{}{}", protocol, addr, args.uri_prefix)
            }
            #[cfg(unix)]
            BindAddr::SocketPath(path) => path.to_string(),
        })
        .collect::<Vec<_>>();

    if urls.len() == 1 {
        output.push_str(&format!("Listening on {}", urls[0]))
    } else {
        let info = urls
            .iter()
            .map(|v| format!("  {v}"))
            .collect::<Vec<String>>()
            .join("\n");
        output.push_str(&format!("Listening on:\n{info}\n"))
    }

    Ok(output)
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C signal handler")
}

#[tauri::command(rename_all = "snake_case")]
fn get_nic_info(
    app: tauri::AppHandle,
    server_handle: tauri::State<Arc<Mutex<Option<oneshot::Sender<()>>>>>,
) -> Vec<String> {
    let args = Args::default();
    let (new_addrs, print_addrs) = check_addrs(&args).unwrap();
    let urls = print_addrs
        .iter()
        .map(|bind_addr| match bind_addr {
            BindAddr::IpAddr(addr) => {
                let addr = match addr {
                    IpAddr::V4(_) => format!("{}", addr),
                    IpAddr::V6(_) => "".parse().unwrap(),
                };
                let protocol = if args.tls_cert.is_some() {
                    "https"
                } else {
                    "http"
                };
                format!("{}://{}{}", protocol, addr, args.uri_prefix)
            }
            #[cfg(unix)]
            BindAddr::SocketPath(path) => path.to_string(),
        })
        .collect::<Vec<_>>()
        .iter()
        .filter(|x| !x.is_empty())
        .map(|x| x.to_string())
        .collect::<Vec<_>>();

    return urls;
    // return Ok(format!("{:?}", urls));
}

#[tauri::command]
fn acquire_permission_android(app: tauri::AppHandle) -> Result<String, String> {
    let api = app.android_fs();

    // pick folder to read and write
    api.acquire_app_manage_external_storage();
    return Ok("done".to_string());
    let selected_folder = api
        .show_manage_dir_dialog(
            None, // Initial location
        )
        .unwrap();

    if let Some(selected_dir_uri) = selected_folder {
        // for entry in api.read_dir(&selected_dir_uri).unwrap() {
        //     match entry {
        //         Entry::File { name, uri, last_modified, len, mime_type, .. } => {
        // return Ok(format!("File: {} - {:?} - {:?} - {}", name, uri, last_modified, len))
        // },
        // Entry::Dir { name, uri, last_modified, .. } => {
        // return Ok(format!("Dir: {} - {:?} - {:?}", name, uri, last_modified))
        // },
        // }
        // }
        debug!("reading /storage/emulated/0/books/index.html");
        debug!("Selected folder: {:?}", &selected_dir_uri);
        let res3 = std::fs::read_to_string("/storage/emulated/0/books/index.html").unwrap();
        debug!("res3: {:?}", res3);

        let res1 = api
            .check_persisted_uri_permission(&selected_dir_uri, PersistableAccessMode::ReadAndWrite)
            .unwrap();
        debug!("res1 {:?}", res1);
        let res2 = api
            .take_persistable_uri_permission(&selected_dir_uri)
            .unwrap();
        debug!("res2 {:?}", res2);
        let persisted_uri_perms = api.get_all_persisted_uri_permissions();
        for permission in persisted_uri_perms {
            debug!("Persisted URI: {:?}", permission.collect::<Vec<_>>());
        }
        // let file_path: tauri_plugin_fs::FilePath = selected_dir_uri.into();
        // let file_path = PathResolver::file_name(selected_dir_uri);
        for entry in api.read_dir(&selected_dir_uri).unwrap() {
            match entry {
                tauri_plugin_android_fs::Entry::File {
                    name,
                    uri,
                    last_modified,
                    len,
                    mime_type,
                    ..
                } => {
                    debug!("***file {:?}", (name, uri, last_modified, len, mime_type));
                }
                tauri_plugin_android_fs::Entry::Dir {
                    name,
                    uri,
                    last_modified,
                    ..
                } => {
                    debug!("***dir {:?}", (name, uri, last_modified));
                }
            }
        }
        return Ok(format!("Selected folder: {:?}", selected_dir_uri));
    }
    return Err("Folder picker canceled".to_string());
}
#[tauri::command]
fn file_picker_example(app: tauri::AppHandle) -> Result<String, String> {
    let api = app.android_fs();
    let mut file_type = "file".to_string(); // Use a `String` instead of a reference

    let mut selected_files = api
        .show_open_file_dialog(
            None,     // Initial location
            &["*/*"], // Target MIME types
            true,     // Allow multiple files
        )
        .unwrap();

    if selected_files.is_empty() {
        Err("File picker canceled".to_string())
    } else {
        if selected_files.len() == 1 {
            let mime_type = api
                .get_mime_type(&selected_files.pop().unwrap())
                .unwrap()
                .unwrap();
            file_type = mime_type; // Assign the `String` value
        } else {
            for uri in selected_files {
                let mime_type = api.get_mime_type(&uri).unwrap().unwrap();
                file_type = mime_type; // Assign the `String` value
                let file_name = api.get_name(&uri).unwrap();

                // Handle file operations if needed
                let file_path: tauri_plugin_fs::FilePath = uri.into();
            }
        }
        Ok(format!("File type: {}", file_type)) // Use the `String` value
    }
} // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(app_handle: tauri::AppHandle, name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}
// use futures::executor::block_on;
#[tauri::command]
fn collect_sys_info() -> String {
    let network_interfaces = NetworkInterface::show().unwrap();
    let mut result: String = "".to_owned();
    for itf in network_interfaces.iter() {
        result.push_str(&format!("{:?}", itf));
    }
    let mut sys = System::new_all();
    sys.refresh_all();
    result.push_str(&format!("=> system:"));
    // RAM and swap information:
    result.push_str(&format!("total memory: {} bytes", sys.total_memory()));
    result.push_str(&format!("used memory : {} bytes", sys.used_memory()));
    result.push_str(&format!("total swap  : {} bytes", sys.total_swap()));
    result.push_str(&format!("used swap   : {} bytes", sys.used_swap()));

    // Display system information:
    result.push_str(&format!("System name:             {:?}", System::name()));
    result.push_str(&format!(
        "System kernel version:   {:?}",
        System::kernel_version()
    ));
    result.push_str(&format!(
        "System OS version:       {:?}",
        System::os_version()
    ));
    result.push_str(&format!(
        "System host name:        {:?}",
        System::host_name()
    ));

    // Number of CPUs:
    result.push_str(&format!("NB CPUs: {}", sys.cpus().len()));

    // Display processes ID, name na disk usage:
    for (pid, process) in sys.processes() {
        result.push_str(&format!(
            "[{pid}] {:?} {:?}",
            process.name(),
            process.disk_usage()
        ));
    }

    // We display all disks' information:
    result.push_str(&format!("=> disks:"));
    let disks = Disks::new_with_refreshed_list();
    for disk in &disks {
        result.push_str(&format!("{disk:?}"));
    }

    return result;
}
async fn daemon(
    app_handle: tauri::AppHandle,
    port: u16,
    my_response: Arc<Message>,
    my_fingerprint: String,
) -> std::io::Result<()> {
    let udp = create_udp_socket(port)?;
    let mut buf = [0; 1024];
    let addr: std::net::Ipv4Addr = "224.0.0.167".parse().unwrap();
    let app_handle_clone = app_handle.clone();
    let peers_store = app_handle_clone.store("peers.json").unwrap();
    peers_store.clear();
    loop {
        let (count, remote_addr) = udp.recv_from(&mut buf).await?;
        let data = buf[..count].to_vec();
        let udp_clone = Arc::clone(&udp);
        let response_clone = my_response.clone();
        let my_fingerprint_clone = my_fingerprint.clone();
        let peers_store_clone = peers_store.clone();
        let app_handle_clone2 = app_handle.clone();

        tauri::async_runtime::spawn(async move {
            if let Ok(parsed_msg) = serde_json::from_slice::<Message>(&data) {
                debug!("daemon received msg: {}", serde_json::to_string(&*response_clone).unwrap());

                let peer_fingerprint = parsed_msg.fingerprint.clone();
                if parsed_msg.fingerprint == my_fingerprint_clone {
                    debug!("skip my own fingerprint");
                    return;
                }
                // if fingerprint is in keys of the store, return
                if let Some(sss) = peers_store_clone.get(&peer_fingerprint) {
                    let peer_info: PeerInfo = serde_json::from_value(sss).unwrap();
                    if peer_info.remote_addrs.contains(&remote_addr) {
                        debug!("skip already registered fingerprint: {}", peer_fingerprint);
                        return;
                    } else {
                        debug!("add new remote address: {:?}", remote_addr);
                        let mut peer_info = peer_info;
                        peer_info.add_remote_addr(remote_addr);
                        peers_store_clone.set(peer_fingerprint, serde_json::json!(peer_info));
                    }
                } else {
                    debug!(
                        "received new multicast message from {:?}: {:?}",
                        remote_addr, parsed_msg
                    );
                    let peer_info = PeerInfo {
                        message: parsed_msg,
                        remote_addrs: vec![remote_addr].into(),
                    };
                    peers_store_clone.set(peer_fingerprint, serde_json::json!(peer_info));
                }
                udp_clone
                        .send_to(
                            &serde_json::to_vec(&*response_clone)
                                    .expect("Failed to serialize Message"),
                            (addr, port),
                        )
                        .await
                        .expect("Send error");
                app_handle_clone2.emit("refresh-peers", ()).unwrap();
            } else {
                log::warn!("Failed to parse incoming multicast message");
            }
        });
    }

    Ok(())
}

#[derive(Default)]
struct AppData {
    addr: &'static str,
    port: u16,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let server_handle = Arc::new(Mutex::new(None::<oneshot::Sender<()>>));
    // use tauri state to manage a vector of String
    let sessions = Mutex::new(Sessions::default());
    #[cfg(debug_assertions)]
    let log_level = log::LevelFilter::Debug;
    #[cfg(not(debug_assertions))]
    let log_level = log::LevelFilter::Info;
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_log::Builder::new().level(log_level).build())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(server_handle.clone())
        .manage(sessions)
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_android_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_view::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_sharetarget::init())
        .setup(|app| {
            // app.manage(Mutex::new(AppData {
            //     addr: "224.0.0.167",
            //     port: 53317,
            // }));
            let settings_store = app.store("settings.json").unwrap();
            let localsend_setting = settings_store.get("localsend");
            let my_fingerprint = match localsend_setting {
                None => {
                    let _my_fingerprint = generate_random_string(FINGERPRINT_LENGTH);
                    info!("no fingerprint found, generate a new one");
                    settings_store.set(
                        "localsend",
                        serde_json::json!({
                        "fingerprint": _my_fingerprint.clone(),
                    }),
                    );
                    _my_fingerprint
                }
                Some(_my_fingerprint) => {
                    _my_fingerprint
                        .get("fingerprint")
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string()
                }
            };
            warn!("my fingerprint : {}", my_fingerprint);
            let port = 53317;

            let my_response = Arc::new(Message {
                alias: my_fingerprint[0..6].to_string(),
                version: "2.1".to_string(),
                device_model: None,
                device_type: None,
                fingerprint: my_fingerprint.clone(),
                port,
                protocol: "http".to_string(),
                download: Some(true),
                announce: None,
            });
            app.manage(Message {
                alias: my_fingerprint[0..6].to_string(),
                version: "2.1".to_string(),
                device_model: None,
                device_type: None,
                fingerprint: my_fingerprint.clone(),
                port,
                protocol: "http".to_string(),
                download: Some(true),
                announce: None,
            });
            let my_response_for_route = Arc::clone(&my_response);
            let my_response_for_announce = Arc::clone(&my_response);
            let my_response_for_daemon = Arc::clone(&my_response);

            let handle_announce = tauri::async_runtime::spawn(periodic_announce(my_response_for_announce));
            let app_handle_axum = app.handle().clone();
            let handle_axum_server = tauri::async_runtime::spawn(async move {
                let axum_app = Router::new()
                    .route(
                        "/api/localsend/v2/register",
                        post(move |Json(payload): Json<Message>| {
                            handler_register(
                                Arc::clone(&my_response_for_route),
                                Json::from(payload),
                            )
                        }),
                    )
                    .route(
                        "/api/localsend/v2/prepare-upload",
                        post(handler_prepare_upload),
                    )
                    .route("/api/localsend/v2/upload", post(handler_upload))
                    .route("/", get(|| async { "This is an axum server" }))
                    .with_state(app_handle_axum);

                let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
                    .await
                    .unwrap();
                axum::serve(listener, axum_app).await.unwrap()
            });
            let app_handle = app.handle().clone();
            let handle_daemon = tauri::async_runtime::spawn(daemon(
                app_handle,
                port,
                my_response_for_daemon,
                my_fingerprint.clone(),
            ));
            // let res = join!(handle_announce, handle_axum_server, handle_daemon);

            // std::thread::spawn(move || block_on(tcc_main()));
            // tauri::async_runtime::spawn(actix_main());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            file_picker_example,
            acquire_permission_android,
            toggle_server,
            get_nic_info,
            collect_sys_info,
            announce_once
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
