use std::io::Write;
use tauri_plugin_fs::FsExt;
#[macro_use]
extern crate log;
use sysinfo::{Disks, System};
use tauri::path::PathResolver;
use tauri::{Emitter, Listener, Manager};
use tauri_plugin_android_fs::{
    AndroidFs, AndroidFsExt, FileUri, InitialLocation, PersistableAccessMode, PrivateDir,
};
use tauri_plugin_sql::{Migration, MigrationKind};
use tauri_plugin_store::StoreExt;
use tokio;
use tokio::sync::oneshot;

mod commands;
mod common;
mod dufs;
mod localsend;
mod assets;

use std::sync::{Arc, Mutex};
#[cfg(feature = "tls")]
use tokio_rustls::{rustls::ServerConfig, TlsAcceptor};

use axum::{
    routing::{get, post},
    Router,
};
use common::{generate_random_string, Message, Sessions, FINGERPRINT_LENGTH};
// use std::io::prelude::*;
use localsend::{
    daemon, handler_prepare_upload, handler_register, handler_upload, periodic_announce,
};
use assets::{proxy_uploads, AppState};
use std::net::SocketAddr;
use std::str::FromStr;
use url::Url;

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
        .post("http://127.0.0.1:53317/api/localsend/v2/register")
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
                // .add_migrations("sqlite:xap.db", migrations)
                .build(),
        )
        .plugin(tauri_plugin_log::Builder::new().level(log_level)
                .level_for("sqlx::query", log::LevelFilter::Info)
                .build())
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
            warn!("readfile11");
            let db_dst = app.path().resolve("", tauri::path::BaseDirectory::Document)?;
            if ! std::fs::exists(db_dst.clone()).unwrap() {
                std::fs::create_dir(db_dst).unwrap();
            }
            warn!("readfile1.1");
            warn!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::Audio).unwrap());
            warn!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::Cache).unwrap());
            warn!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::Config).unwrap());
            warn!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::Data).unwrap());
            warn!("2");
            warn!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::LocalData).unwrap());
            warn!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::Document).unwrap());
            warn!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::Download).unwrap());
            warn!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::Picture).unwrap());
            warn!("3");
            warn!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::Resource).unwrap());
            warn!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::AppConfig).unwrap());
            warn!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::AppData).unwrap());
            warn!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::AppLocalData).unwrap());
            warn!("4");
            warn!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::AppCache).unwrap());
            warn!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::AppLog).unwrap());
            warn!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::Home).unwrap());
            warn!("{:?}", app.path().resolve("xap.db", tauri::path::BaseDirectory::Cache).unwrap());
            let db_src = app.path().resolve("res/xap.db", tauri::path::BaseDirectory::Resource)?;
            let db_dst = app.path().resolve("xap.db", tauri::path::BaseDirectory::Document)?;
            warn!("readfile: src {:?}", db_src.clone());
            warn!("readfile: dst {:?}", db_dst.clone());
            if cfg!(target_os = "android") {
                // this SQL copy logic currently does not work on Android, patched it in sql plugin rust code
                warn!("readfile 1");
                let scope = app.fs_scope();
                let android_fs_api = app.android_fs();
                scope.allow_directory(app.path().resolve("", tauri::path::BaseDirectory::Document).unwrap(), false);
                // scope.allow_directory("/path/to/directory", false);
                // dbg!(scope.allowed());
                // warn!("{:?}", scope.allowed());
                let src_path = tauri_plugin_fs::FilePath::Path(db_src.clone());
                warn!("readfile 2: {:?}", src_path);
                let db_file_content = app.fs().read(src_path).unwrap();
                // warn!("readfile 4: {:?}", db_file_content.clone());
                warn!("{:?}", db_dst.as_path());
                // let file = tauri_plugin_fs::OpenOptions::new().write(true).open(db_dst.as_path());
                let p =  tauri_plugin_fs::FilePath::Path(db_dst);
                let uri: FileUri = p.into();
                let mut file: std::fs::File = android_fs_api.open_file(&uri, tauri_plugin_android_fs::FileAccessMode::WriteTruncate)
                        .unwrap();
                warn!("writeall");
                // let file: std::fs::File = api.open_file(&uri, FileAccessMode::WriteTruncate)?;
                // let mut file_opened = std::fs::OpenOptions::new().write(true).open(db_dst.as_path()).unwrap();
                // warn!("copying bundled sqlite");
                file.write_all(&db_file_content);
                warn!("done");
            } else {
                // let db_file_content = std::fs::File::open(&db_src).unwrap();
                if !std::path::Path::new(&db_dst.clone()).exists() {
                    warn!("copy bundled sqlite");
                } else {
                    warn!("overrite existing")
                }
                std::fs::copy(db_src.as_path(), db_dst.as_path()).unwrap();
                warn!("done");
            }

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
            let message = Message {
                alias: my_fingerprint[0..6].to_string(),
                version: "2.1".to_string(),
                device_model: None,
                device_type: None,
                fingerprint: my_fingerprint.clone(),
                port,
                protocol: "http".to_string(),
                download: Some(true),
                announce: None,
            };
            let my_response = Arc::new(message.clone());
            app.manage(message);
            let my_response_for_route = Arc::clone(&my_response);
            let my_response_for_announce = Arc::clone(&my_response);
            let my_response_for_daemon = Arc::clone(&my_response);

            let _handle_announce =
                tauri::async_runtime::spawn(periodic_announce(my_response_for_announce));
            let app_handle_axum = app.handle().clone();
            let axum_app_state = Arc::new(AppState {
                app_handle: app_handle_axum,
                client: reqwest::Client::new(),
            });
            let _handle_axum_server = tauri::async_runtime::spawn(async move {
                let axum_app = Router::new()
                    .route("/uploads/{*path}", get(proxy_uploads))
                    .route("/api/localsend/v2/register", post(handler_register))
                    .route(
                        "/api/localsend/v2/prepare-upload",
                        post(handler_prepare_upload),
                    )
                    .route("/api/localsend/v2/upload", post(handler_upload))
                    .route("/", get(|| async { "This is an axum server" }))
                        .with_state(axum_app_state);

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
            let _handle_daemon = tauri::async_runtime::spawn(daemon(
                app_handle,
                port,
                my_response_for_daemon,
                my_fingerprint.clone(),
            ));
            // let res = join!(_handle_announce, _handle_axum_server, _handle_daemon);

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
