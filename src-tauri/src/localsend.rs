use crate::common::{
    create_udp_socket, generate_random_string, ChatMessage, Message, PeerInfo, PrepareUploadParams,
    PrepareUploadRequest, PrepareUploadRequestAndSessionId, Session, Sessions, TokenAndUploadFile,
    UploadQuery, FILE_TOKEN_LENGTH, SESSION_LENGTH,
};
use crate::reverse_proxy::AppState;
use axum::extract::{ConnectInfo, Query, State};
use axum::Json;
use futures_util::TryStreamExt;
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};
use tauri_plugin_store::{JsonValue, StoreExt};
use tokio::fs::File;
use tokio::io::BufWriter;
use tokio_util::io::StreamReader;

pub async fn handler_register(
    State(state): State<Arc<AppState>>,
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    // my_response: Arc<Message>,
    Json(payload): Json<Message>,
) -> () {
    let app_handle = state.app_handle.clone();
    let peers_store = app_handle.store("peers.json").unwrap();
    let settings_store = app_handle.store("settings.json").unwrap();
    let localsend_setting = settings_store.get("localsend");
    let my_fingerprint = localsend_setting
        .unwrap()
        .get("fingerprint")
        .unwrap()
        .to_string();
    // ) -> Json<Message> {
    // Here you can process the payload as needed
    info!("axum register_handler received message: {:?}", payload);
    let peer_fingerprint = payload.fingerprint.clone();
    let my_fingerprint_clone = my_fingerprint.clone();
    if payload.fingerprint == my_fingerprint_clone {
        info!("skip my own fingerprint");
        return;
    }
    // if fingerprint is in keys of the store, return
    if let Some(peer_value) = peers_store.get(&peer_fingerprint) {
        let peer_info: PeerInfo = serde_json::from_value(peer_value).unwrap();
        if peer_info.remote_addrs.contains(&remote_addr) {
            info!("skip already registered fingerprint: {}", peer_fingerprint);
            return;
        } else {
            info!("add new remote address: {:?}", remote_addr);
            let mut peer_info = peer_info;
            peer_info.add_remote_addr(remote_addr);
            peers_store.set(peer_fingerprint, serde_json::json!(peer_info));
        }
    } else {
        info!(
            "received new multicast message from {:?}: {:?}",
            remote_addr, payload
        );
        let peer_info = PeerInfo {
            message: payload,
            remote_addrs: vec![remote_addr].into(),
        };
        peers_store.set(peer_fingerprint, serde_json::json!(peer_info));
    }
    app_handle.emit("refresh-peers", ()).unwrap();

    ()
}

// #[axum::debug_handler]
#[allow(non_snake_case)]
pub async fn handler_prepare_upload(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PrepareUploadParams>,
    Json(payload): Json<PrepareUploadRequest>,
) -> Json<HashMap<String, JsonValue>> {
    info!("axum handler_prepare_upload Payload: {:?}", payload);
    info!(
        "axum handler_prepare_upload Received request with params: {:?}",
        params
    );
    let app_handle = state.app_handle.clone();
    // waiting the state of whether user has accepted the request for 10s, if not, return error
    let sessionId = generate_random_string(SESSION_LENGTH);
    {
        let sessions_state = app_handle.state::<Mutex<Sessions>>();
        let mut sessions = sessions_state.lock().unwrap();
        sessions.sessions.insert(
            sessionId.clone(),
            Session {
                accepted: false,
                userFeedback: false,
                finished: false,
                fileIdtoTokenAndUploadFile: HashMap::new(),
            },
        );
        drop(sessions);
    }
    app_handle
        .emit(
            "prepare-upload",
            PrepareUploadRequestAndSessionId {
                sessionId: sessionId.clone(),
                prepareUploadRequest: payload.clone(),
            },
        )
        .unwrap();

    tokio::select! {
        _ = tokio::time::sleep(tokio::time::Duration::from_secs(10)) => {
            // Timeout after 10 seconds
            info!("Timeout waiting for user to accept the request");
            return Json({
                let mut response = HashMap::new();
                response.insert("error".to_string(), serde_json::to_value("Timeout waiting for user to accept the request").unwrap());
                response
            });
        },
        res = async {
           loop {
                let session = {
                    let sessions_state = app_handle.state::<Mutex<Sessions>>();
                    let sessions = sessions_state.lock().unwrap();
                    info!("handler_prepare_upload: acquired lock on sessions");
                    let session = sessions.sessions.get(&sessionId).cloned();
                    drop(sessions); // Explicitly drop the MutexGuard here
                    session
                };

                if let Some(session) = session {
                    if session.userFeedback {
                        let mut files_tokens = HashMap::new();
                        let mut files_tokenAndUploadFiles = HashMap::new();
                        if session.accepted {
                            for (fileId, file) in &payload.files.files {
                            let token = generate_random_string(FILE_TOKEN_LENGTH);
                                files_tokens.insert(fileId.clone(), token.clone());
                                files_tokenAndUploadFiles.insert(fileId.clone(), TokenAndUploadFile{
                                        token: token,
                                        uploadFile: file.clone(),
                                    });
                                };
                            {
                                let sessions_state = app_handle.state::<Mutex<Sessions>>();
                                info!("handler_prepare_upload: acquiring lock on sessions");
                                let mut sessions = sessions_state.lock().unwrap();
                                info!("handler_prepare_upload: acquired lock on sessions");
                                sessions.sessions.insert(sessionId.clone(), Session{
                                    accepted: true,
                                    userFeedback: true,
                                    finished: false,
                                    fileIdtoTokenAndUploadFile: files_tokenAndUploadFiles,
                                });
                                drop(sessions);
                                info!("handler_prepare_upload: released lock on sessions1");
                            }
                            return Json({
                                    let mut response = HashMap::new();
                                    response.insert("sessionId".to_string(), serde_json::to_value(sessionId.clone()).unwrap());
                                    response.insert(
                                        "files".to_string(),
                                        serde_json::to_value(files_tokens).unwrap(),
                                    );
                                    response
                                })
                            }
                             else {
                                {
                                    let sessions_state = app_handle.state::<Mutex<Sessions>>();
                                    info!("handler_prepare_upload: acquiring lock on sessions");
                                    let mut sessions = sessions_state.lock().unwrap();
                                    info!("handler_prepare_upload: acquired lock on sessions");
                                    sessions.sessions.remove(&sessionId);
                                    drop(sessions);
                                    info!("handler_prepare_upload: released lock on sessions2");
                                }

                                return Json({
                                    let mut response = HashMap::new();
                                    response.insert("error".to_string(), serde_json::to_value("User rejected the request").unwrap());
                                    response
                                });
                            }
                        }
                    }
                    info!("Waiting 500ms for user feedback...");
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
            } => {
            return res
        },
    }
}

// #[axum::debug_handler]
#[allow(non_snake_case)]
pub async fn handler_chat(
    State(state): State<Arc<AppState>>,
    Json(chat_message): Json<ChatMessage>,
) -> Json<anyhow::Result<(), String>> {
    info!("handler_chat: received chat message: {:?}", chat_message);

    let app_handle = state.app_handle.clone();

    // Forward the chat message to the frontend
    match app_handle.emit("chat-message-received", chat_message.clone()) {
        Ok(_) => {
            info!("handler_chat: emitted chat-message-received event");

            // Call the handle_incoming_chat_message command to process the message
            match crate::chat_commands::handle_incoming_chat_message(app_handle, chat_message) {
                Ok(_) => Json(Ok(())),
                Err(e) => {
                    error!("Error handling incoming chat message: {}", e);
                    Json(Err(format!("Error handling incoming chat message: {}", e)))
                }
            }
        },
        Err(e) => {
            error!("Error emitting chat-message-received event: {}", e);
            Json(Err(format!("Error emitting chat-message-received event: {}", e)))
        }
    }
}

pub async fn handler_upload(
    State(state): State<Arc<AppState>>,
    Query(query_params): Query<UploadQuery>,
    body: axum::body::Body,
) -> Json<anyhow::Result<(), String>> {
    info!(
        "axum handler_prepare_upload query_params: {:?}",
        query_params
    );
    info!("handler_upload: entering");
    let mut filename: String;
    let mut savingDir: String;
    let app_handle = state.app_handle.clone();
    {
        let settings_store = app_handle.store("settings.json").unwrap();
        let localsend_setting = settings_store.get("localsend");
        savingDir = match localsend_setting {
            Some(localsend_setting) => {
                let localsend_setting: HashMap<String, String> =
                    serde_json::from_value(localsend_setting).unwrap();
                localsend_setting
                    .get("savingDir")
                    .unwrap_or(&"/tmp".to_string())
                    .trim_end_matches("/")
                    .to_string()
            }
            None => "/tmp".to_string(),
        };
        let sessions_state = app_handle.state::<Mutex<Sessions>>();
        info!("handler_upload: acquiring lock on sessions");
        let sessions = sessions_state.lock().unwrap();
        info!("handler_upload: acquired lock on sessions");
        info!("sessions cloned (before) {:?}", sessions.clone());
        let session = sessions.sessions.get(&query_params.sessionId).cloned();

        if let Some(session) = session.clone() {
            if session.accepted && session.userFeedback {
                if let Some(fileId_to_tokenAndUploadFile) = session
                    .fileIdtoTokenAndUploadFile
                    .get(query_params.fileId.as_str())
                {
                    if fileId_to_tokenAndUploadFile.token != query_params.token {
                        return Json(Err("Invalid token".to_string()));
                    } else {
                        filename = fileId_to_tokenAndUploadFile.uploadFile.fileName.clone();
                    }
                } else {
                    return Json(Err("Invalid fileId".to_string()));
                }
            } else {
                return Json(Err(
                    "Session not accepted or user feedback not received".to_string()
                ));
            }
        } else {
            return Json(Err("Session not found".to_string()));
        }
        info!("handler_upload: released lock on sessions3");
        drop(sessions);
    }

    let res = async {
        info!("{:?}", query_params);
        let path = format!("{}/{}", savingDir, filename);
        info!("saving to path: {}", path);
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
        }
    }
}

pub async fn periodic_announce(
    app_handle: tauri::AppHandle,
    my_response: Arc<Message>) -> std::io::Result<()> {
    let port = 53317;
    let udp_socket = app_handle.state::<Arc<tokio::net::UdpSocket>>();
    let addr: std::net::Ipv4Addr = "224.0.0.167".parse().unwrap();
    let mut count = 0;
    let announce_interval = 10;
    loop {
        info!("announce sequence {}", count);
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
        udp_socket.send_to(
            &serde_json::to_vec(&my_response_new).expect("Failed to serialize Message"),
            (addr, port),
        )
        .await
        .unwrap_or_else(|e| { warn!("Failed to send multicast message: {}", e); 0 });
        tokio::time::sleep(std::time::Duration::from_secs(announce_interval)).await;
        count += 1;
    }
    Ok(())
}

// localsend.rs - Modified `daemon` to start synchronization upon peer discovery
pub async fn daemon(
    app_handle: tauri::AppHandle,
    port: u16,
    my_response: Arc<Message>,
    my_fingerprint: String,
) -> std::io::Result<()> {
    let udp_socket = app_handle.state::<Arc<tokio::net::UdpSocket>>();
    let mut buf = [0; 1024];
    let addr: std::net::Ipv4Addr = "224.0.0.167".parse().unwrap();
    let peers_store = app_handle.store("peers.json").unwrap();
    let client_insecure = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    loop {
        let (count, remote_addr) = udp_socket.recv_from(&mut buf).await?;
        let data = buf[..count].to_vec();
        let response_clone = my_response.clone();
        let my_fingerprint_clone = my_fingerprint.clone();
        let peers_store_clone = peers_store.clone();
        let app_handle_clone = app_handle.clone();
        let client_insecure_clone = client_insecure.clone();

        tauri::async_runtime::spawn(async move {
            // Other LocalSend members will listen to messages from the multicast group,
            // when an announcement message comes, will reply with own information.
            if let Ok(parsed_msg) = serde_json::from_slice::<Message>(&data) {
                let remote_port = parsed_msg.port;
                info!(
                    "daemon received msg: {}",
                    serde_json::to_string(&*response_clone).unwrap()
                );

                // Skip if it's my own message
                if parsed_msg.fingerprint == my_fingerprint_clone {
                    info!("skip my own fingerprint");
                    return;
                }

                // Update or register peer
                if let Some(peer_value) = peers_store_clone.get(&parsed_msg.fingerprint) {
                    let mut peer_info: PeerInfo = serde_json::from_value(peer_value).unwrap();
                    if !peer_info.remote_addrs.contains(&remote_addr) {
                        peer_info.add_remote_addr(remote_addr);
                        peers_store_clone
                            .set(parsed_msg.fingerprint.clone(), serde_json::json!(peer_info));
                    }
                } else {
                    info!(
                        "received new multicast message from {:?}: {:?}",
                        remote_addr, parsed_msg
                    );
                    let peer_info = PeerInfo {
                        message: parsed_msg.clone(),
                        remote_addrs: vec![remote_addr].into(),
                    };
                    peers_store_clone
                        .set(parsed_msg.fingerprint.clone(), serde_json::json!(peer_info));
                }
                app_handle_clone.emit("refresh-peers", ()).unwrap();
                let register_url = format!("{}://{}/api/localsend/v2/register", parsed_msg.protocol, remote_addr);
                warn!("post to register_url: {}", register_url);
                let res = client_insecure_clone
                        .post(register_url)
                        .json(&*response_clone)
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
                
                if parsed_msg.device_model.unwrap() == "localshare_device" {
                    info!("peer is localshare, start syncing files");
                    // Initiate file sync with peer
                    let peer_protocol = parsed_msg.protocol;
                    let peer_address =
                        format!("{}://{}:{}", peer_protocol, remote_addr, remote_port);

                    if let Err(e) = sync_files_with_peer(
                        &client_insecure_clone,
                        peer_address,
                        app_handle_clone.clone(),
                    )
                    .await
                    {
                        info!("File sync with peer failed: {}", e);
                    }
                }
            } else {
                log::warn!("Failed to parse incoming multicast message");
            }
        });
    }
}

async fn sync_files_with_peer(
    client_insecure: &reqwest::Client,
    peer_address: String,
    app_handle: tauri::AppHandle,
) -> anyhow::Result<()> {
    // Retrieve and list local files
    let cache_dir = app_handle
        .path()
        .resolve("assets", tauri::path::BaseDirectory::AppCache)
        .unwrap();

    let mut local_files = vec![];
    let mut entries = tokio::fs::read_dir(&cache_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        if entry.file_type().await?.is_file() {
            local_files.push(entry.file_name().to_string_lossy().to_string());
        }
    }

    // Get file list from peer
    let peer_files: Vec<String> = client_insecure
        .get(format!("{}/api/files", peer_address))
        .send()
        .await?
        .json()
        .await?;

    // Identify missing files to download from peer
    let files_to_fetch: Vec<String> = peer_files
        .clone()
        .into_iter()
        .filter(|file| !local_files.contains(file))
        .collect();

    for file in files_to_fetch {
        let response = client_insecure
            .get(format!("{}/api/files/download/{}", peer_address, file))
            .send()
            .await?;

        let content = response.bytes().await?;
        let file_path = cache_dir.join(&file);
        tokio::fs::write(file_path, content).await?;
    }

    // Identify files to upload to peer
    let files_to_upload: Vec<String> = local_files
        .into_iter()
        .filter(|file| !peer_files.contains(file))
        .collect();

    for file in files_to_upload {
        let file_path = cache_dir.join(&file);
        let content = tokio::fs::read(&file_path).await?;
        client_insecure
            .post(format!("{}/api/files/upload", peer_address))
            .body(content)
            .send()
            .await?;
    }

    Ok(())
}
