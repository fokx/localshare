#[macro_use]
extern crate log;
use futures::StreamExt;
use network_interface::NetworkInterface;
use network_interface::NetworkInterfaceConfig;
use sysinfo::{Disks, System};
use tauri::path::PathResolver;
use tauri::Manager;
use tauri_plugin_android_fs::{
    AndroidFs, AndroidFsExt, FileUri, InitialLocation, PersistableAccessMode, PrivateDir,
};
use tokio;
// use tokio::task::JoinHandle;
// use tauri::async_runtime::TokioJoinHandle;
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

use axum::{
    extract::Request, handler::HandlerWithoutStateExt, http::StatusCode, routing::get, Router,
};
use std::net::SocketAddr;
use tower::ServiceExt;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

async fn dufs_main(shutdown_rx: oneshot::Receiver<()>, port: usize) -> Result<()> {
    let cmd = build_cli();
    let matches = cmd.get_matches();
    if let Some(generator) = matches.get_one::<Shell>("completions") {
        let mut cmd = build_cli();
        print_completions(*generator, &mut cmd);
        return Ok(());
    }
    let mut args = Args::parse(matches)?;
    // logger::init(args.log_file.clone()).map_err(|e| anyhow!("Failed to init logger, {e}"))?;
    // let (new_addrs, print_addrs) = check_addrs(&args)?;
    // args.addrs = new_addrs;
    let running = Arc::new(AtomicBool::new(true));
    // let listening = print_listening(&args, &print_addrs)?;
    // let listener = create_listener(SocketAddr::new("0.0.0.0".parse()?, 4804))
    //         .with_context(|| format!("Failed to bind"))?;
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
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
                        eprintln!("accept error: {}", e);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                };
                eprintln!("incomming connection accepted: {}", peer_addr);
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
                        eprintln!("Error serving connection: {:?}", e);
                    }
                    eprintln!("connection dropped: {}", peer_addr);
                });
            },
            _ = signal.as_mut() => {
                drop(listener);
                eprintln!("graceful shutdown signal received");
                // stop the accept loop
                break;
            }
            // _ = async {
            //         handle.await.unwrap();
            //         // Help the rust type inferencer out
            //         Ok::<_,std::io::Error>(())
            //     } => {}
            // _ = rx => {
            //     println!("terminating async task");
            //     running.store(false, Ordering::SeqCst);
            //     println!("async task terminated");
            // },
        }
    }

    tokio::select! {
        _ = graceful.shutdown() => {
            eprintln!("all connections gracefully closed");
        },
        _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
            eprintln!("timed out wait for all connections to close");
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
fn toggle_server(
    app: tauri::AppHandle,
    server_handle: tauri::State<Arc<Mutex<Option<oneshot::Sender<()>>>>>,
    port: usize,
) -> Result<String, String> {
    println!("using port: {}", port);
    let mut state_locked = server_handle.lock().unwrap();

    if let Some(shutdown_tx) = state_locked.take() {
        // Stop the server
        println!("Stopping server");
        let _ = shutdown_tx.send(()); // Send the shutdown signal
        return Ok("stopped".to_string());
    } else {
        // Start the server
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        // let runtime = tokio::runtime::Runtime::new().unwrap();
        let join_handle = tauri::async_runtime::spawn(dufs_main(shutdown_rx, port));
        // runtime.spawn(async move {
        //     actix_main(shutdown_rx).await;
        // });
        *state_locked = Some(shutdown_tx);
        return Ok("started".to_string());
    }
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
                    IpAddr::V6(_) => "".parse().unwrap()
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
        .collect::<Vec<_>>().iter().filter(|x| !x.is_empty()).map(|x| x.to_string()).collect::<Vec<_>>();

    return urls;
    // return Ok(format!("{:?}", urls));
}

#[tauri::command]
fn folder_picker_example(app: tauri::AppHandle) -> Result<String, String> {
    let api = app.android_fs();

    // pick folder to read and write
    api.acquire_manage_external_storage();
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
        println!("reading /storage/emulated/0/books/index.html");
        println!("Selected folder: {:?}", &selected_dir_uri);
        let res3 = std::fs::read_to_string("/storage/emulated/0/books/index.html").unwrap();
        println!("res3: {:?}", res3);

        let res1 = api
            .check_persisted_uri_permission(&selected_dir_uri, PersistableAccessMode::ReadAndWrite)
            .unwrap();
        println!("res1 {:?}", res1);
        let res2 = api
            .take_persistable_uri_permission(&selected_dir_uri)
            .unwrap();
        println!("res2 {:?}", res2);
        let persisted_uri_perms = api.get_all_persisted_uri_permissions();
        for permission in persisted_uri_perms {
            println!("Persisted URI: {:?}", permission.collect::<Vec<_>>());
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
                    println!("***file {:?}", (name, uri, last_modified, len, mime_type));
                }
                tauri_plugin_android_fs::Entry::Dir {
                    name,
                    uri,
                    last_modified,
                    ..
                } => {
                    println!("***dir {:?}", (name, uri, last_modified));
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let server_handle = Arc::new(Mutex::new(None::<oneshot::Sender<()>>));
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(server_handle.clone())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_android_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_view::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // std::thread::spawn(move || block_on(tcc_main()));
            // tauri::async_runtime::spawn(actix_main());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            file_picker_example,
            folder_picker_example,
            toggle_server,
            get_nic_info,
            collect_sys_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
