use crate::common::{
    create_udp_socket, generate_random_color, generate_random_string, ChatHistory, ChatMessage,
    ChatSession, ChatSessions, Files, Message, PeerInfo, PrepareUploadRequest, Session, Sessions,
    UploadFile, FILEID_LENGTH,
};
use network_interface::{NetworkInterface, NetworkInterfaceConfig};
use std::collections::HashMap;
use std::io::Read;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use sysinfo::{Disks, System};
use tauri::{Emitter, Manager, Window};
use tauri_plugin_android_fs::{AndroidFsExt, PersistableAccessMode};
use tauri_plugin_fs::{FsExt, OpenOptions};
use tauri_plugin_store::{JsonValue, StoreExt};
use tokio::sync::oneshot;
use url::Url;

#[tauri::command(rename_all = "snake_case")]
pub fn get_nic_info(
    _app: tauri::AppHandle,
    _server_handle: tauri::State<Arc<Mutex<Option<oneshot::Sender<()>>>>>,
) -> Vec<String> {
    let args = crate::dufs::Args::default();
    let (_new_addrs, print_addrs) = crate::dufs::check_addrs(&args).unwrap();
    let urls = print_addrs
        .iter()
        .map(|bind_addr| match bind_addr {
            crate::dufs::BindAddr::IpAddr(addr) => {
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
            crate::dufs::BindAddr::SocketPath(path) => path.to_string(),
        })
        .collect::<Vec<_>>()
        .iter()
        .filter(|x| !x.is_empty())
        .map(|x| x.to_string())
        .collect::<Vec<_>>();

    return urls;
    // return Ok(format!("{:?}", urls));
}

#[allow(non_snake_case)]
#[tauri::command(rename_all = "snake_case")]
pub async fn send_file_to_peer(
    app_handle: tauri::AppHandle,
    my_response: tauri::State<'_, Message>,
    peer_fingerprint: String,
    files: Vec<String>,
) -> anyhow::Result<String, String> {
    info!("send_file_to_peer");
    info!("peer fingerprint: {}", peer_fingerprint);
    info!("files: {:?}", files);
    let peers_store = app_handle.store("peers.json").unwrap();
    let peers_store_clone = peers_store.clone();
    // let peer_fingerprint = peer_fingerprint.clone();
    let mut remote_addrs;
    // let remote_port;
    let remote_protocol;
    if let Some(peer_value) = peers_store_clone.get(&peer_fingerprint) {
        let peer_info: PeerInfo = serde_json::from_value(peer_value).unwrap();
        remote_addrs = peer_info.remote_addrs;
        // remote_port = peer_info.message.port;
        remote_protocol = peer_info.message.protocol.clone();
        warn!("remote remote_protocol: {}", remote_protocol.clone());
    } else {
        let msg = format!("peer {} not found in peers store", peer_fingerprint);
        info!("{}", msg);
        return Err(msg.to_string());
    }
    // this client's security is decided by peer
    let client_maybe_insecure = if remote_protocol.as_str() == "https" {
        reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap()
    } else {
        reqwest::Client::new()
    };
    let mut files_map = HashMap::new();
    let mut fileId_to_fullpath_map = HashMap::new();
    for file in files {
        let fileId = generate_random_string(FILEID_LENGTH);
        let filename = app_handle.path().file_name(&file.clone()).unwrap();
        info!("filename: {}", filename);
        let filesize = if cfg!(target_os = "android") {
            let fs_api = app_handle.fs();
            // The Tauri FS plugin doesn't support read metadata from Rust side,
            // temporarily mitigate this by read the file, which is inefficient
            // let path = tauri_plugin_fs::FilePath::Url(Url::from_str(file.as_str()).unwrap());
            // let options = OpenOptions::default();
            // let file = fs_api.open(path, options).unwrap(); // called `Result::unwrap()` on an `Err` value: Custom { kind: Other, error: "failed to open file: Bad mode: " }
            // let metadata = file.metadata().unwrap();
            // let size = metadata.len();
            let path = tauri_plugin_fs::FilePath::Url(Url::from_str(&*file).unwrap());
            let mut file = fs_api.read(path).unwrap();
            file.len() as u64
        } else {
            let file_metatdata = std::fs::metadata(file.clone());
            let filesize: u64 = match file_metatdata {
                Ok(metadata) => {
                    let size = metadata.len();
                    info!("file size: {}", size);
                    size
                }
                Err(e) => {
                    info!("error getting file size: {:?}", e);
                    9999
                }
            };
            filesize
        };
        files_map.insert(
            fileId.clone(),
            UploadFile {
                id: fileId.clone(),
                fileName: filename,
                size: filesize,
                fileType: "application/octet-stream".to_string(),
                sha256: None,
                preview: None,
                metadata: None,
            },
        );
        fileId_to_fullpath_map.insert(fileId.clone(), file.clone());
    }
    let request = PrepareUploadRequest {
        info: Message {
            alias: my_response.alias.clone(),
            version: my_response.version.clone(),
            device_model: my_response.device_model.clone(),
            device_type: my_response.device_type.clone(),
            fingerprint: my_response.fingerprint.clone(),
            port: my_response.port,
            protocol: my_response.protocol.clone(),
            download: my_response.download,
            announce: None,
        },
        files: Files {
            files: files_map.clone(),
        },
    };
    let remote_host = remote_addrs.get(0).unwrap().clone();
    let remote_host_53317 = SocketAddr::new(remote_host.ip(), 53317);
    remote_addrs.push_front(remote_host_53317);
    for remote_addr in remote_addrs {
        let client_maybe_insecure_clone = client_maybe_insecure.clone();
        info!("remote host: {}", remote_addr);
        let res = client_maybe_insecure_clone
            .post(format!(
                "{}://{}/api/localsend/v2/prepare-upload",
                remote_protocol, remote_addr
            ))
            .json(&request)
            .send()
            .await;
        match res {
            Ok(response) => {
                info!("peer reply to prepare-upload: {:?}", response);
                // log response content
                let status = response.status();
                info!("peer reply to prepare-upload status: {:?}", status);
                if status.is_success() {
                    let response_text = response.text().await.unwrap();
                    info!("peer reply to prepare-upload response: {:?}", response_text);
                    let response_json: HashMap<String, JsonValue> =
                        serde_json::from_str(&response_text).unwrap();
                    info!(
                        "peer reply to prepare-upload response json: {:?}",
                        response_json
                    );
                    if let Some(sessionId) = response_json.get("sessionId") {
                        let sessionId = sessionId.as_str().unwrap();
                        info!("peer reply to prepare-upload sessionId: {:?}", sessionId);
                        let filesIdToToken = response_json.get("files").unwrap();
                        for (fileId, _file) in &files_map {
                            let token = filesIdToToken.get(fileId.clone()).unwrap();
                            let token = token.as_str().unwrap();
                            let client_maybe_insecure_2 = client_maybe_insecure.clone();
                            // POST /api/localsend/v2/upload?sessionId=mySessionId&fileId=someFileId&token=someFileToken
                            //
                            // Request
                            //
                            // Binary data
                            // read file body and send
                            let fullpath = fileId_to_fullpath_map.get(fileId).unwrap();
                            warn!("fullpath: {:?}", fullpath);
                            let file_binary = if cfg!(target_os = "android") {
                                let fs_api = app_handle.fs();
                                // let android_fs_api = app_handle.android_fs();
                                // let options = OpenOptions::default();
                                let path = tauri_plugin_fs::FilePath::Url(
                                    Url::from_str(fullpath).unwrap(),
                                );
                                let mut file = fs_api.read(path).unwrap();
                                file
                            } else {
                                warn!("read using std::fs");
                                std::fs::read(fullpath).unwrap()
                            };
                            let url = format!(
                                "{}://{}/api/localsend/v2/upload?sessionId={}&fileId={}&token={}",
                                remote_protocol, remote_addr, sessionId, fileId, token
                            );
                            info!("url: {}", url);
                            let res = client_maybe_insecure_2
                                .post(url)
                                .body(file_binary)
                                .send()
                                .await;
                            match res {
                                Ok(response) => {
                                    info!("peer reply to upload: {:?}", response);
                                    let status = response.status();
                                    info!("peer reply to upload status: {:?}", status);
                                }
                                Err(e) => {
                                    info!("error uploadr: {:?}", e);
                                }
                            };
                        }
                    }
                    // when transfer to one address succeeds, won't try another address
                    break;
                } else {
                    info!(
                        "peer reply to prepare-upload error: {:?}",
                        response.text().await.unwrap()
                    );
                }
            }
            Err(e) => {
                info!("error prepare-uploadr: {:?}", e);
            }
        }
    }

    Ok("ok".to_string())
}

#[allow(non_snake_case)]
#[tauri::command]
pub fn handle_incoming_request(
    app_handle: tauri::AppHandle,
    sessionId: String,
    accept: bool,
) -> anyhow::Result<String, String> {
    info!("handle_incoming_request: entering");
    let sessions_state = app_handle.state::<Mutex<Sessions>>();
    info!("handle_incoming_request: acquiring lock on sessions");
    let mut sessions = sessions_state.lock().unwrap();
    info!("handle_incoming_request: acquired lock on sessions");
    info!("sessions cloned (before) {:?}", sessions.clone());
    let session = sessions.sessions.get(&sessionId).cloned();
    if let Some(session) = session {
        if accept {
            sessions.sessions.insert(
                sessionId,
                Session {
                    accepted: true,
                    userFeedback: true,
                    finished: false,
                    fileIdtoTokenAndUploadFile: session.fileIdtoTokenAndUploadFile.clone(),
                },
            );
        } else {
            sessions.sessions.insert(
                sessionId,
                Session {
                    accepted: false,
                    userFeedback: true,
                    finished: false,
                    fileIdtoTokenAndUploadFile: HashMap::new(),
                },
            );
        }
    }
    info!("sessions cloned (after) {:?}", sessions.clone());
    drop(sessions);
    // let mut session = session.unwrap();
    // let mut files_tokens = HashMap::new();
    // for (fileId, _) in &payload.files.files {
    //     let token = format!("token_for_{}", fileId); // Replace it with actual token generation logic
    //     files_tokens.insert(fileId.clone(), token);
    // }
    //
    // sessions.sessions.insert(sessionId.clone(), files_tokens.clone());

    Ok("ok".to_string())
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_peers(
    app_handle: tauri::AppHandle,
) -> anyhow::Result<Vec<PeerInfo>, String> {
    info!("get_peers");

    let peers_store = app_handle.store("peers.json").unwrap();
    let mut peers = Vec::new();

    for key in peers_store.keys() {
        if let Some(peer_value) = peers_store.get(&key) {
            let peer_info: PeerInfo = serde_json::from_value(peer_value)
                .map_err(|e| format!("Failed to parse peer info: {}", e))?;
            peers.push(peer_info);
        }
    }

    Ok(peers)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn announce_once(
    app_handle: tauri::AppHandle,
    my_response: tauri::State<'_, Message>,
) -> anyhow::Result<String, String> {
    let port = 53317;
    let addr: std::net::Ipv4Addr = "224.0.0.167".parse().unwrap();
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
    info!("announce_once: {:?}", my_response_new);
    let udp_socket = app_handle.state::<Arc<tokio::net::UdpSocket>>();
    udp_socket.send_to(
        &serde_json::to_vec(&my_response_new).expect("Failed to serialize Message"),
        (addr, port),
    )
    .await
    .unwrap_or_else(|e| { warn!("Failed to send multicast message: {}", e); 0 });
    Ok("announced".to_string())
}
#[tauri::command(rename_all = "snake_case")]
pub fn toggle_server(
    _app: tauri::AppHandle,
    server_handle: tauri::State<Arc<Mutex<Option<oneshot::Sender<()>>>>>,
    server_port: usize,
    serve_path: String,
    require_auth: bool,
    auth_user: String,
    auth_passwd: String,
    allow_upload: bool,
) -> anyhow::Result<String, String> {
    info!("using server_port: {}", server_port);
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
        let _join_handle = tauri::async_runtime::spawn(crate::dufs::main(
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

#[tauri::command(rename_all = "snake_case")]
pub fn acquire_permission_android(app: tauri::AppHandle) -> anyhow::Result<String, String> {
    let api = app.android_fs();

    // pick a folder to read and write
    let _res = api
        .acquire_app_manage_external_storage()
        .unwrap_or_else(|_| {
            info!("Permission acquire_app_manage_external_storage not granted");
            ()
        });
    return Ok("done".to_string());
    // let selected_folder = api
    //     .show_manage_dir_dialog(
    //         None, // Initial location
    //     )
    //     .unwrap();
    //
    // if let Some(selected_dir_uri) = selected_folder {
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
    // info!("reading /storage/emulated/0/books/index.html");
    // info!("Selected folder: {:?}", &selected_dir_uri);
    // let res3 = std::fs::read_to_string("/storage/emulated/0/books/index.html").unwrap();
    // info!("res3: {:?}", res3);
    //
    // let res1 = api
    //     .check_persisted_uri_permission(&selected_dir_uri, PersistableAccessMode::ReadAndWrite)
    //     .unwrap();
    // info!("res1 {:?}", res1);
    // let res2 = api
    //     .take_persistable_uri_permission(&selected_dir_uri)
    //     .unwrap();
    // info!("res2 {:?}", res2);
    // let persisted_uri_perms = api.get_all_persisted_uri_permissions();
    // for permission in persisted_uri_perms {
    //     info!("Persisted URI: {:?}", permission.collect::<Vec<_>>());
    // }
    // let file_path: tauri_plugin_fs::FilePath = selected_dir_uri.into();
    // let file_path = PathResolver::file_name(selected_dir_uri);
    //     for entry in api.read_dir(&selected_dir_uri).unwrap() {
    //         match entry {
    //             tauri_plugin_android_fs::Entry::File {
    //                 name,
    //                 uri,
    //                 last_modified,
    //                 len,
    //                 mime_type,
    //                 ..
    //             } => {
    //                 info!("***file {:?}", (name, uri, last_modified, len, mime_type));
    //             }
    //             tauri_plugin_android_fs::Entry::Dir {
    //                 name,
    //                 uri,
    //                 last_modified,
    //                 ..
    //             } => {
    //                 info!("***dir {:?}", (name, uri, last_modified));
    //             }
    //         }
    //     }
    //     return Ok(format!("Selected folder: {:?}", selected_dir_uri));
    // }
    // return Err("Folder picker canceled".to_string());
}
#[tauri::command(rename_all = "snake_case")]
pub fn file_picker_example(app: tauri::AppHandle) -> anyhow::Result<String, String> {
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
                let _file_name = api.get_name(&uri).unwrap();

                // Handle file operations if needed
                let _file_path: tauri_plugin_fs::FilePath = uri.into();
            }
        }
        Ok(format!("File type: {}", file_type)) // Use the `String` value
    }
}

#[tauri::command(rename_all = "snake_case")]
pub fn greet(_app_handle: tauri::AppHandle, name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}
#[tauri::command(rename_all = "snake_case")]
pub fn collect_sys_info() -> String {
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

#[tauri::command]
pub fn start_oauth_server(window: Window) -> Result<u16, String> {
    let config = tauri_plugin_oauth::OauthConfig {
        ports: Some(vec![4810, 4811, 4812]),
        response: Some("Login successful. You can close this window.".into()),
    };

    tauri_plugin_oauth::start_with_config(config, move |url| {
        // Send the OAuth URL back to the frontend
        let _ = window.emit("oauth_redirect", url);
    })
            .map_err(|err| err.to_string())
}


