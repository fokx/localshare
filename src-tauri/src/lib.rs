use anyhow::Result;
use std::io::Write;
use tauri_plugin_fs::FsExt;
#[macro_use]
extern crate log;
use hyper_util::rt::{TokioExecutor, TokioIo};
use sysinfo::{Disks, System};
use tauri::path::PathResolver;
use tauri::{Emitter, Listener, Manager};
use tauri_plugin_android_fs::{
    AndroidFs, AndroidFsExt, FileUri, InitialLocation, PersistableAccessMode, PrivateDir, UriType,
};
use tauri_plugin_sql::{Migration, MigrationKind};
use tauri_plugin_store::StoreExt;
use tokio;
use tokio::sync::{oneshot, Notify};
use zstd::decode_all;

mod login;
mod commands;
mod common;
mod dufs;

mod localsend;
mod reverse_proxy;
mod socks2http;
mod tuicc;
mod chat_commands;
use sha2::{Digest, Sha256};
use std::fs::read;

use std::sync::{Arc, Mutex};
#[cfg(feature = "tls")]
use tokio_rustls::{rustls::ServerConfig, TlsAcceptor};

use axum::{
    routing::{self, get, post},
    Router,
};
use common::{generate_random_string, Message, Sessions, FINGERPRINT_LENGTH};
// use std::io::prelude::*;
use crate::rustls::crypto::CryptoProvider;
use localsend::{
    daemon, handler_chat, handler_prepare_upload, handler_register, handler_upload, periodic_announce,
};
use rcgen::{date_time_ymd, CertificateParams, DistinguishedName, DnType, KeyPair, SanType};
use reverse_proxy::{
    download_file, list_files, proxy_all_requests, proxy_uploads, upload_file, AppState,
};
use rustls::pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer};
use std::fs;
use std::net::SocketAddr;
use std::str::FromStr;
use std::{fs::File, io::BufReader};
use tokio::net::TcpListener;
use tokio_rustls::rustls::{self, ServerConfig};
use tokio_rustls::TlsAcceptor;
use tower::Service;
use url::Url;

#[tokio::test]
async fn client_test() -> std::io::Result<()> {
    // cargo test -- --nocapture
    // https://stackoverflow.com/questions/25106554/why-doesnt-println-work-in-rust-unit-tests
    let my_fingerprint = generate_random_string(FINGERPRINT_LENGTH);
    info!("test client fingerprint : {}", my_fingerprint);
    let port = 53317;
    let my_response = Arc::new(Message {
        alias: my_fingerprint[0..6].to_string(),
        version: "2.1".to_string(),
        device_model: Some("localshare_device".to_string()),
        device_type: Some("unimplemented".to_string()),
        fingerprint: my_fingerprint.clone(),
        port,
        protocol: "https".to_string(),
        download: Some(true),
        announce: Some(false),
    });

    let my_response_for_announce = Arc::clone(&my_response);
    let my_response_clone = Arc::clone(&my_response);

    periodic_announce(my_response_for_announce).await?;
    // POST to "/api/localsend/v2/register"
    let client = reqwest::Client::new();
    let res = client
        .post("http://127.0.0.1:53317/api/localsend/v2/register")
        .json(&*my_response_clone)
        .send()
        .await;
    match res {
        Ok(response) => {
            info!("Response: {:?}", response);
        }
        Err(e) => {
            info!("Error: {:?}", e);
        }
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");
    let server_handle = Arc::new(Mutex::new(None::<oneshot::Sender<()>>));
    // use tauri state to manage a vector of String
    let sessions = Mutex::new(Sessions::default());
    #[cfg(debug_assertions)]
    let log_level = log::LevelFilter::Debug;
    #[cfg(not(debug_assertions))]
    let log_level = log::LevelFilter::Info;
    let migrations = vec![
        // Define your migrations here
        Migration {
            version: 1,
            description: "create_initial_tables",
            sql: include_str!("../migrations/0000_strong_black_bird.sql"),
            kind: MigrationKind::Up,
        },
    ];
    tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_os::init())
        .plugin(
            tauri_plugin_sql::Builder::default()
                // .add_migrations("sqlite:xap.db", migrations)
                .build(),
        )
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log_level)
                .level_for("sqlx::query", log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(server_handle.clone())
        .manage(sessions)
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_android_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_oauth::init())

        .plugin(tauri_plugin_view::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_sharetarget::init())
        // .plugin(tauri_plugin_mic_recorder::init())
        .setup(|app| {
            info!("readfile11");
            let db_dst = app
                .path()
                .resolve("", tauri::path::BaseDirectory::Document)?;
            if !std::fs::exists(db_dst.clone()).unwrap() {
                std::fs::create_dir(db_dst).unwrap();
            }

            // info!("readfile1.1");
            // info!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::Audio).unwrap());
            // info!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::Cache).unwrap());
            // info!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::Config).unwrap());
            // info!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::Data).unwrap());
            // info!("2");
            // info!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::LocalData).unwrap());
            // info!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::Document).unwrap());
            // info!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::Download).unwrap());
            // info!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::Picture).unwrap());
            // info!("3");
            // info!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::Resource).unwrap());
            // info!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::AppConfig).unwrap());
            // info!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::AppData).unwrap());
            // info!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::AppLocalData).unwrap());
            // info!("4");
            // info!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::AppCache).unwrap());
            // info!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::AppLog).unwrap());
            // info!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::Home).unwrap());
            // info!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::Cache).unwrap());
            // cd src-tauri/res; zstdmt -19 xap.db -o xap.db.zst
            let db_src = app
                .path()
                .resolve("res/xap.db.zst", tauri::path::BaseDirectory::Resource)?;
            let db_dst = app
                .path()
                .resolve("xap.db", tauri::path::BaseDirectory::Document)?;
            info!("readfile: src {:?}", db_src.clone());
            info!("readfile: dst {:?}", db_dst.clone());
            if cfg!(target_os = "android") {
                // this SQL copy logic currently does not work on Android, patched it in sql plugin rust code
                info!("readfile 1");
                let scope = app.fs_scope();
                let android_fs_api = app.android_fs();
                scope.allow_directory(
                    app.path()
                        .resolve("", tauri::path::BaseDirectory::Document)
                        .unwrap(),
                    false,
                );
                // scope.allow_directory("/path/to/directory", false);
                // dbg!(scope.allowed());
                // info!("{:?}", scope.allowed());
                let src_path = tauri_plugin_fs::FilePath::Path(db_src.clone());
                info!("readfile 2: {:?}", src_path);
                let compressed_content = app.fs().read(src_path).unwrap();
                let db_file_content = decode_all(&compressed_content[..]).unwrap();
                info!("{:?}", db_dst.as_path());
                let p = tauri_plugin_fs::FilePath::Path(db_dst);
                let uri: FileUri = p.into();
                if android_fs_api.get_uri_type(&uri).unwrap() == UriType::NotFound {
                    let mut file: std::fs::File = android_fs_api
                        .open_file(&uri, tauri_plugin_android_fs::FileAccessMode::WriteTruncate)
                        .unwrap();
                    info!("writeall decompressed content");
                    file.write_all(&db_file_content).unwrap();
                    info!("done");
                } else {
                    info!("sqlite already exists, will not overrite");
                }
            } else {
                // let db_file_content = std::fs::File::open(&db_src).unwrap();
                if !std::path::Path::new(&db_dst.clone()).exists() {
                    let compressed_content = std::fs::read(db_src.as_path()).unwrap();
                    let db_file_content = decode_all(&compressed_content[..]).unwrap();
                    std::fs::write(db_dst.as_path(), db_file_content).unwrap();
                    info!("done");
                    info!("copy bundled sqlite");
                } else {
                    info!("sqlite already exists, will not overrite")
                }
            }

            let settings_store = app.store("settings.json").unwrap();
            let localsend_setting = settings_store.get("localsend");

            let certs_dst_dir = app
                .path()
                .resolve("certs", tauri::path::BaseDirectory::AppLocalData)?;
            if !std::fs::exists(certs_dst_dir.clone()).unwrap() {
                std::fs::create_dir(certs_dst_dir).unwrap();
            }
            let cer_dst = app
                .path()
                .resolve("certs/cer.pem", tauri::path::BaseDirectory::AppLocalData)?;
            let cer_der_dst = app
                .path()
                .resolve("certs/cer.der", tauri::path::BaseDirectory::AppLocalData)?;
            let key_dst = app
                .path()
                .resolve("certs/key.pem", tauri::path::BaseDirectory::AppLocalData)?;
            let cert_hash: String;
            if !std::fs::exists(cer_der_dst.clone()).unwrap() {
                let mut params: CertificateParams = Default::default();
                params.not_before = date_time_ymd(1975, 1, 1);
                params.not_after = date_time_ymd(4096, 1, 1);
                let subject_alt_names = generate_random_string(FINGERPRINT_LENGTH);
                info!("cert SAN: {}", subject_alt_names);
                params.subject_alt_names = vec![SanType::DnsName(
                    rcgen::Ia5String::from_str(&subject_alt_names).unwrap(),
                )];

                let key_pair = KeyPair::generate()?;
                let cert = params.self_signed(&key_pair)?;
                let mut hasher = Sha256::new();
                hasher.update(cert.der());
                let result = hasher.finalize();
                cert_hash = hex::encode(result);
                info!("hash der: {}", cert_hash);
                let pem_serialized = cert.pem();
                let pem = pem::parse(&pem_serialized)?;
                let der_serialized = pem.contents();
                info!("{pem_serialized}");
                info!("{}", key_pair.serialize_pem());
                // std::fs::create_dir_all("certs/")?;
                std::fs::write(cer_dst.clone(), pem_serialized.as_bytes())?;
                std::fs::write(cer_der_dst, der_serialized)?;
                std::fs::write(key_dst.clone(), key_pair.serialize_pem().as_bytes())?;
                // std::fs::write("key.der", key_pair.serialize_der())?;
            } else {
                let cert_der = std::fs::read(cer_der_dst)?;
                let mut hasher = Sha256::new();
                hasher.update(cert_der);
                let result = hasher.finalize();
                cert_hash = hex::encode(result);
                info!("hash der: {}", cert_hash);
            }
            let my_fingerprint = match localsend_setting {
                None => {
                    let _my_fingerprint = cert_hash;
                    info!("no fingerprint saved, use a new one derived from certificate");
                    settings_store.set(
                        "localsend",
                        serde_json::json!({
                            "fingerprint": _my_fingerprint.clone(),
                            "savingDir": "/storage/emulated/0/Download".to_string(),
                        }),
                    );
                    _my_fingerprint
                }
                Some(setting) => setting
                    .get("fingerprint")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string(),
            };
            info!("my fingerprint : {}", my_fingerprint);
            let port = 53317;
            let message = Message {
                alias: my_fingerprint[0..6].to_string(),
                version: "2.1".to_string(),
                device_model: Some("localshare".to_string()),
                device_type: None,
                fingerprint: my_fingerprint.clone(),
                port,
                protocol: "http".to_string(),
                download: Some(true),
                announce: None,
            };
            let my_response = Arc::new(message.clone());
            app.manage(message);
            let my_response_for_announce = Arc::clone(&my_response);
            let my_response_for_daemon = Arc::clone(&my_response);

            let _handler_announce =
                tauri::async_runtime::spawn(periodic_announce(my_response_for_announce));
            let app_handle_axum = app.handle().clone();
            let socks5_url = reqwest::Url::parse("socks5h://127.0.0.1:4807").unwrap();

            // Build the client with SOCKS5 proxy
            // let reqwest_client = reqwest::Client::new();
            let reqwest_client = reqwest::Client::builder()
                // .user_agent("localshareapp") // platform specific UA?
                .proxy(reqwest::Proxy::all(socks5_url).unwrap())
                .cookie_store(true)
                .deflate(true)
                .zstd(true)
                .brotli(true)
                .build()
                .unwrap();

            let axum_app_state = Arc::new(AppState {
                app_handle: app_handle_axum,
                client: reqwest_client,
            });
            let axum_app_state_https = axum_app_state.clone();

            let _handler_axum_https_server = tauri::async_runtime::spawn(async move {
                let axum_app = Router::new()
                    .route("/api/localsend/v2/register", post(handler_register))
                    .route(
                        "/api/localsend/v2/prepare-upload",
                        post(handler_prepare_upload),
                    )
                    .route("/api/localsend/v2/upload", post(handler_upload))
                    .route("/api/localsend/v2/chat", post(handler_chat))
                    .route("/api/files", get(list_files))
                    .route("/api/files/download/{filename}", get(download_file))
                    .route("/api/files/upload/{filename}", post(upload_file))
                    .route("/", get(|| async { "This is an HTTPS Axum server" }))
                    .with_state(axum_app_state_https);
                if my_response.clone().protocol == "http" {
                    warn!("binding on 53317 http");
                    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
                        .await
                        .unwrap();
                    warn!("binding on 53317 http finished");
                    axum::serve(
                        listener,
                        axum_app.into_make_service_with_connect_info::<SocketAddr>(),
                    )
                    .await
                    .unwrap();
                } else {
                    let certs = CertificateDer::pem_file_iter(cer_dst.clone())
                        .unwrap()
                        .collect::<Result<Vec<_>, _>>()
                        .unwrap();
                    let key = PrivateKeyDer::from_pem_file(key_dst.clone()).unwrap();
                    let mut config = ServerConfig::builder()
                        .with_no_client_auth()
                        .with_single_cert(certs, key)
                        .expect("Failed to configure TLS");
                    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
                    let tls_acceptor = TlsAcceptor::from(Arc::new(config));
                    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
                    warn!("binding on 53317 https");
                    let tcp_listener = TcpListener::bind(addr).await.unwrap();
                    warn!("binding on 53317 https finished");
                    loop {
                        let tower_service = axum_app.clone();
                        let tls_acceptor = tls_acceptor.clone();
                        // Wait for new tcp connection
                        let (cnx, addr) = tcp_listener.accept().await.unwrap();
                        tokio::spawn(async move {
                            // Wait for tls handshake to happen
                            let Ok(stream) = tls_acceptor.accept(cnx).await else {
                                error!("error during tls handshake connection from {}", addr);
                                return;
                            };
                            // Hyper has its own `AsyncRead` and `AsyncWrite` traits and doesn't use tokio.
                            // `TokioIo` converts between them.
                            let stream = TokioIo::new(stream);
                            // Hyper also has its own `Service` trait and doesn't use tower. We can use
                            // `hyper::service::service_fn` to create a hyper `Service` that calls our app through
                            // `tower::Service::call`.
                            let hyper_service = hyper::service::service_fn(
                                move |request: axum::extract::Request<hyper::body::Incoming>| {
                                    // We have to clone `tower_service` because hyper's `Service` uses `&self` whereas
                                    // tower's `Service` requires `&mut self`.
                                    //
                                    // We don't need to call `poll_ready` since `Router` is always ready.
                                    tower_service.clone().call(request)
                                },
                            );
                            let ret =
                                hyper_util::server::conn::auto::Builder::new(TokioExecutor::new())
                                    .serve_connection_with_upgrades(stream, hyper_service)
                                    .await;
                            if let Err(err) = ret {
                                warn!("error serving connection from {}: {}", addr, err);
                            }
                        });
                    }
                }
            });
            let notify_handler_axum_http_server = Arc::new(Notify::new());
            let notify_clone_handler_axum_http_server =
                Arc::clone(&notify_handler_axum_http_server);

            let _handler_axum_http_server = tauri::async_runtime::spawn(async move {
                let axum_app = Router::new()
                    .route("/uploads/{*path}", get(proxy_uploads))
                    .route(
                        "/.well-known/localshare",
                        get(|| async { "This is an HTTP Axum server" }),
                    )
                    .route("/{*path}", routing::any(proxy_all_requests))
                    .route("/", routing::any(proxy_all_requests))
                    .with_state(axum_app_state);
                warn!("binding on 4805");
                let listener = match tokio::net::TcpListener::bind("127.0.0.1:4805").await {
                    Ok(listener) => {
                        // Notify that the server has successfully started
                        notify_clone_handler_axum_http_server.notify_one();
                        listener
                    }
                    Err(e) => {
                        log::error!("Failed to start HTTP server: {}", e);
                        return;
                    }
                };
                warn!("binding on 4805 finished");

                axum::serve(
                    listener,
                    axum_app.into_make_service_with_connect_info::<SocketAddr>(),
                )
                .await
                .unwrap()
            });

            let app_handle = app.handle().clone();
            let _handler_daemon = tauri::async_runtime::spawn(daemon(
                app_handle,
                port,
                my_response_for_daemon,
                my_fingerprint.clone(),
            ));
            // let res = join!(_handle_announce, _handle_axum_server, _handle_daemon);

            // std::thread::spawn(move || block_on(tcc_main()));
            // tauri::async_runtime::spawn(actix_main());
            #[cfg(debug_assertions)] // only include this code on info builds
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
                // let url = Url::parse("https://xjtu.app:443")?;
                // let tauri_url = WebviewUrl::External(url);
                // let webview_window =
                //     tauri::WebviewWindowBuilder::new(app, "label", tauri_url)
                //             .proxy_url(Url::parse("socks5://127.0.0.1:4807")?)
                //             // .devtools(true)
                //             .build()?;
                // webview_window.open_devtools();

                // WebviewWindowBuilder::new(
                //     "webview window", WebviewUrl::External(url::Url::parse("https://127.0.0.1:4802")?)),
                //         // .proxy_url(Url::parse("socks5://127.0.0.1:4807")?) // may cause white screen
                //         .build()?;

                // let webview = window.add_child( // Available on desktop and crate feature unstable only.
                //                                 webview_builder,
                //                                 tauri::LogicalPosition::new(0, 0),
                //                                 window.inner_size().unwrap(),
                // );
            }
            let _handler_tuicc = tauri::async_runtime::spawn(crate::tuicc::main());
            let _handler_socks2http = tauri::async_runtime::spawn(crate::socks2http::main());
            warn!("waiting for web server to start");
            let result = tauri::async_runtime::block_on(async {
                tokio::time::timeout(
                    tokio::time::Duration::from_secs(5),
                    notify_handler_axum_http_server.notified(),
                )
                .await
            });

            if result.is_err() {
                return Err("HTTP server failed to start within 5 seconds.".into());
            } else {
                warn!(" web server started");
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::file_picker_example,
            commands::acquire_permission_android,
            commands::toggle_server,
            commands::get_nic_info,
            commands::collect_sys_info,
            commands::announce_once,
            commands::handle_incoming_request,
            commands::send_file_to_peer,
            commands::start_oauth_server,
            login::login_with_provider,
            chat_commands::send_chat_message,
            chat_commands::handle_incoming_chat_message,
            chat_commands::get_chat_history,
            chat_commands::get_chat_sessions,
            chat_commands::mark_messages_as_read
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
