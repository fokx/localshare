#[macro_use]
extern crate log;
use futures::StreamExt;
use log::{debug, error, info, trace, warn};
use sysinfo::{Disks, System};
use tauri::path::PathResolver;
use tauri::{Emitter, Listener, Manager};
use tauri_plugin_android_fs::{
    AndroidFs, AndroidFsExt, FileUri, InitialLocation, PersistableAccessMode, PrivateDir,
};
use tauri_plugin_sql::{Migration, MigrationKind};

// use tokio::task::JoinHandle;
// use tauri::async_runtime::TokioJoinHandle;
use tauri_plugin_log::{Target, TargetKind};
use tauri_plugin_store::{JsonValue, StoreExt};
use tokio;
use tokio::sync::oneshot;

mod common;
mod localsend;
mod commands;
mod dufs;

use anyhow::{anyhow, Context, Result};
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


use axum::extract::{ConnectInfo, Path};
use common::{generate_random_string, create_udp_socket, Message, PeerInfo, Sessions, FINGERPRINT_LENGTH};
use localsend::{
    periodic_announce,
    handler_register,
    handler_prepare_upload,
    handler_upload,
    daemon
};
// use std::io::prelude::*;
use futures::{Stream, TryStreamExt};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::io::{self, Error, Write};
use tokio::{fs::File, io::BufWriter};
use tokio_util::io::StreamReader;
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
        .plugin(tauri_plugin_os::init())
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations("sqlite:test.db", migrations)
                .build(),
        )
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

            let handle_announce =
                tauri::async_runtime::spawn(periodic_announce(my_response_for_announce));
            let app_handle_axum = app.handle().clone();
            let handle_axum_server = tauri::async_runtime::spawn(async move {
                let axum_app = Router::new()
                    .route(
                        "/api/localsend/v2/register",
                        post(handler_register),
                        // post(move |Json(payload): Json<Message>| {
                        //     handler_register(
                        //         Arc::clone(&my_response_for_route),
                        //         Json::from(payload),
                        //     )
                        // }),
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
                axum::serve(
                    listener,
                    axum_app.into_make_service_with_connect_info::<SocketAddr>(),
                )
                .await
                .unwrap()
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
            #[cfg(debug_assertions)] // only include this code on debug builds
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
                // let url = Url::parse("https://xjtu.app:443")?;
                // let tauri_url = WebviewUrl::External(url);
                // let webview_window =
                //     tauri::WebviewWindowBuilder::new(app, "label", tauri_url)
                //             .proxy_url(Url::parse("socks5://127.0.0.1:4801")?)
                //             // .devtools(true)
                //             .build()?;
                // webview_window.open_devtools();

                // WebviewWindowBuilder::new(
                //     "webview window", WebviewUrl::External(url::Url::parse("https://127.0.0.1:4802")?)),
                //         // .proxy_url(Url::parse("socks5://127.0.0.1:4801")?) // may cause white screen
                //         .build()?;

                // let webview = window.add_child( // Available on desktop and crate feature unstable only.
                //                                 webview_builder,
                //                                 tauri::LogicalPosition::new(0, 0),
                //                                 window.inner_size().unwrap(),
                // );
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
