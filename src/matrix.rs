use std::sync::mpsc::Sender;

use chrono::Local;
use log::info;
use matrix_sdk::{
    config::SyncSettings,
    ruma::events::room::message::{MessageType, OriginalSyncRoomMessageEvent},
    Room, RoomState,
};

use crate::{ClientMessage, Message};

async fn on_room_message(
    event: OriginalSyncRoomMessageEvent,
    room: Room,
    sender: Sender<ClientMessage>,
) {
    // We only want to log text messages in joined rooms.
    if room.state() != RoomState::Joined {
        return;
    }
    let MessageType::Text(text_content) = &event.content.msgtype else {
        return;
    };

    let room_name = match room.display_name().await {
        Ok(room_name) => room_name.to_string(),
        Err(error) => {
            println!("Error getting room display name: {error}");
            // Let's fallback to the room ID.
            room.room_id().to_string()
        }
    };

    println!("[{room_name}] {}: {}", event.sender, text_content.body);

    let message = Message {
        sender: event.sender.to_string(),
        contents: text_content.body.clone(),
        timestamp: Local::now(),
    };

    // Send the message to the Iced application
    if let Err(e) = sender.send(ClientMessage::NewMessage(message)) {
        println!("Error sending message to Iced application: {}", e);
    }
}

pub async fn login(
    homeserver_url: String,
    username: String,
    password: String,
    sender: Sender<ClientMessage>,
) -> anyhow::Result<(matrix_sdk::Client, String)> {
    info!("Connecting to homeserver {}", &homeserver_url);

    let client = matrix_sdk::Client::builder()
        .homeserver_url(homeserver_url)
        .build()
        .await?;

    client
        .matrix_auth()
        .login_username(&username, &password)
        .initial_device_display_name(env!("CARGO_PKG_NAME"))
        .send()
        .await?;

    info!("logged in as {username}");

    let sync_settings = SyncSettings::default();
    let sync_token = client.sync_once(sync_settings.clone()).await?.next_batch;

    client.add_event_handler(move |event, room| {
        let sender = sender.clone();
        async move {
            on_room_message(event, room, sender).await;
        }
    });

    client.sync(sync_settings).await?;

    Ok((client, sync_token))
}
