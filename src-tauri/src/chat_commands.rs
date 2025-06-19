use crate::common::{
    generate_random_color, generate_random_string, ChatHistory, ChatMessage, ChatSession,
    ChatSessions, Message, PeerInfo,
};
use std::net::SocketAddr;
use tauri::{Emitter, Manager};
use log::{debug, info, warn, error};
use tauri_plugin_store::StoreExt;

#[tauri::command]
pub async fn send_chat_message(
    app_handle: tauri::AppHandle,
    my_response: tauri::State<'_, Message>,
    peer_fingerprint: String,
    content: String,
) -> anyhow::Result<String, String> {
    info!("send_chat_message");
    info!("peer fingerprint: {}", peer_fingerprint);
    info!("content: {}", content);

    let peers_store = app_handle.store("peers.json").unwrap();
    let chat_store = app_handle.store("chat.json").unwrap();

    // Get peer information
    let mut remote_addrs;
    let remote_protocol;
    let peer_alias;

    if let Some(peer_value) = peers_store.get(&peer_fingerprint) {
        let peer_info: PeerInfo = serde_json::from_value(peer_value).unwrap();
        remote_addrs = peer_info.remote_addrs;
        remote_protocol = peer_info.message.protocol.clone();
        peer_alias = peer_info.message.alias.clone();
        info!("remote protocol: {}", remote_protocol.clone());
    } else {
        let msg = format!("peer {} not found in peers store", peer_fingerprint);
        info!("{}", msg);
        return Err(msg.to_string());
    }

    // Create a new chat message
    let message_id = generate_random_string(16);
    let chat_message = ChatMessage {
        id: message_id.clone(),
        sender_fingerprint: my_response.fingerprint.clone(),
        sender_alias: my_response.alias.clone(),
        receiver_fingerprint: peer_fingerprint.clone(),
        content: content.clone(),
        timestamp: std::time::SystemTime::now(),
        read: true, // Sender's messages are always read
    };

    // Update chat history
    let chat_history_key = format!("history_{}", peer_fingerprint);
    let mut chat_history = if let Some(history) = chat_store.get(&chat_history_key) {
        serde_json::from_value(history).unwrap_or_else(|_| ChatHistory::default())
    } else {
        ChatHistory::default()
    };

    chat_history.messages.push(chat_message.clone());
    chat_store.set(chat_history_key, serde_json::json!(chat_history));

    // Update chat sessions
    let mut chat_sessions = if let Some(sessions) = chat_store.get("sessions") {
        serde_json::from_value(sessions).unwrap_or_else(|_| ChatSessions::default())
    } else {
        ChatSessions::default()
    };

    // Create or update the chat session
    if !chat_sessions.sessions.contains_key(&peer_fingerprint) {
        chat_sessions.sessions.insert(
            peer_fingerprint.clone(),
            ChatSession {
                peer_fingerprint: peer_fingerprint.clone(),
                peer_alias: peer_alias.clone(),
                last_message: Some(chat_message.clone()),
                unread_count: 0,
                color: generate_random_color(),
            },
        );
    } else {
        let session = chat_sessions.sessions.get_mut(&peer_fingerprint).unwrap();
        session.last_message = Some(chat_message.clone());
    }

    chat_store.set("sessions", serde_json::json!(chat_sessions));

    // Send the message to the peer
    let client_maybe_insecure = if remote_protocol.as_str() == "https" {
        reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap()
    } else {
        reqwest::Client::new()
    };

    let remote_host = remote_addrs.get(0).unwrap().clone();
    let remote_host_53317 = SocketAddr::new(remote_host.ip(), 53317);
    remote_addrs.push_front(remote_host_53317);

    for remote_addr in remote_addrs {
        let client_maybe_insecure_clone = client_maybe_insecure.clone();
        info!("remote host: {}", remote_addr);

        let res = client_maybe_insecure_clone
            .post(format!(
                "{}://{}/api/localsend/v2/chat",
                remote_protocol, remote_addr
            ))
            .json(&chat_message)
            .send()
            .await;

        match res {
            Ok(response) => {
                info!("peer reply to chat message: {:?}", response);
                let status = response.status();
                if status.is_success() {
                    return Ok("Message sent successfully".to_string());
                }
            }
            Err(e) => {
                info!("error sending chat message: {:?}", e);
                // Continue trying other addresses
            }
        }
    }

    // If we get here, we couldn't send the message to any of the peer's addresses
    Err("Failed to send message to peer".to_string())
}

#[tauri::command]
pub fn handle_incoming_chat_message(
    app_handle: tauri::AppHandle,
    chat_message: ChatMessage,
) -> anyhow::Result<String, String> {
    info!("handle_incoming_chat_message");
    info!("message: {:?}", chat_message);

    let chat_store = app_handle.store("chat.json").unwrap();

    // Update chat history
    let peer_fingerprint = chat_message.sender_fingerprint.clone();
    let chat_history_key = format!("history_{}", peer_fingerprint);
    let mut chat_history = if let Some(history) = chat_store.get(&chat_history_key) {
        serde_json::from_value(history).unwrap_or_else(|_| ChatHistory::default())
    } else {
        ChatHistory::default()
    };

    // Add the message to history
    let mut new_message = chat_message.clone();
    new_message.read = false; // Mark as unread initially
    chat_history.messages.push(new_message.clone());
    chat_store.set(chat_history_key, serde_json::json!(chat_history));

    // Update chat sessions
    let mut chat_sessions = if let Some(sessions) = chat_store.get("sessions") {
        serde_json::from_value(sessions).unwrap_or_else(|_| ChatSessions::default())
    } else {
        ChatSessions::default()
    };

    // Create or update the chat session
    if !chat_sessions.sessions.contains_key(&peer_fingerprint) {
        chat_sessions.sessions.insert(
            peer_fingerprint.clone(),
            ChatSession {
                peer_fingerprint: peer_fingerprint.clone(),
                peer_alias: chat_message.sender_alias.clone(),
                last_message: Some(new_message.clone()),
                unread_count: 1,
                color: generate_random_color(),
            },
        );
    } else {
        let session = chat_sessions.sessions.get_mut(&peer_fingerprint).unwrap();
        session.last_message = Some(new_message.clone());
        session.unread_count += 1;
    }

    chat_store.set("sessions", serde_json::json!(chat_sessions));

    // Notify the frontend about the new message
    app_handle.emit("chat-message-received", chat_message).unwrap();

    Ok("Message received".to_string())
}


#[tauri::command]
pub fn get_chat_sessions(
    app_handle: tauri::AppHandle,
) -> anyhow::Result<ChatSessions, String> {
    info!("get_chat_sessions");

    let chat_store = app_handle.store("chat.json").unwrap();

    if let Some(sessions) = chat_store.get("sessions") {
        let chat_sessions: ChatSessions = serde_json::from_value(sessions)
            .map_err(|e| format!("Failed to parse chat sessions: {}", e))?;
        Ok(chat_sessions)
    } else {
        Ok(ChatSessions::default())
    }
}

#[tauri::command]
pub fn mark_messages_as_read(
    app_handle: tauri::AppHandle,
    peer_fingerprint: String,
) -> anyhow::Result<String, String> {
    info!("mark_messages_as_read");
    info!("peer fingerprint: {}", peer_fingerprint);

    let chat_store = app_handle.store("chat.json").unwrap();

    // Update chat history
    let chat_history_key = format!("history_{}", peer_fingerprint);
    if let Some(history) = chat_store.get(&chat_history_key) {
        let mut chat_history: ChatHistory = serde_json::from_value(history)
            .map_err(|e| format!("Failed to parse chat history: {}", e))?;

        // Mark all messages as read
        for message in &mut chat_history.messages {
            if message.sender_fingerprint == peer_fingerprint {
                message.read = true;
            }
        }

        chat_store.set(chat_history_key, serde_json::json!(chat_history));
    }

    // Update chat sessions
    if let Some(sessions) = chat_store.get("sessions") {
        let mut chat_sessions: ChatSessions = serde_json::from_value(sessions)
            .map_err(|e| format!("Failed to parse chat sessions: {}", e))?;

        if let Some(session) = chat_sessions.sessions.get_mut(&peer_fingerprint) {
            session.unread_count = 0;
        }

        chat_store.set("sessions", serde_json::json!(chat_sessions));
    }

    Ok("Messages marked as read".to_string())
}

#[tauri::command]
pub fn get_chat_history(
    app_handle: tauri::AppHandle,
    peer_fingerprint: String,
) -> anyhow::Result<ChatHistory, String> {
    info!("get_chat_history");
    info!("peer fingerprint: {}", peer_fingerprint);

    let chat_store = app_handle.store("chat.json").unwrap();
    let chat_history_key = format!("history_{}", peer_fingerprint);

    if let Some(history) = chat_store.get(&chat_history_key) {
        let chat_history: ChatHistory = serde_json::from_value(history)
                .map_err(|e| format!("Failed to parse chat history: {}", e))?;
        Ok(chat_history)
    } else {
        Ok(ChatHistory::default())
    }
}
