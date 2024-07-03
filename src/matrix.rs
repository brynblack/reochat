use std::{
    path::{Path, PathBuf},
    sync::mpsc::Sender,
};

use chrono::Local;
use log::info;
use matrix_sdk::{
    config::SyncSettings,
    matrix_auth::MatrixSession,
    ruma::{
        api::client::filter::FilterDefinition,
        events::room::message::{MessageType, OriginalSyncRoomMessageEvent},
    },
    Client, Error, LoopCtrl, Room, RoomState,
};
use rand::{distributions::Alphanumeric, rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::{ClientMessage, Message};

#[derive(Debug, Serialize, Deserialize)]
struct ClientSession {
    homeserver: String,
    db_path: PathBuf,
    passphrase: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct FullSession {
    client_session: ClientSession,
    user_session: MatrixSession,

    #[serde(skip_serializing_if = "Option::is_none")]
    sync_token: Option<String>,
}

pub(crate) struct Credentials {
    pub username: String,
    pub password: String,
}

pub async fn run(credentials: Credentials) -> anyhow::Result<(Client, Option<String>)> {
    let data_dir = Path::new("data");
    let session_file = data_dir.join("session");

    let (client, sync_token) = if session_file.exists() {
        restore_session(&session_file).await?
    } else {
        (login(credentials, &data_dir, &session_file).await?, None)
    };

    Ok((client, sync_token))
}

pub async fn start_event_loop(
    client: Client,
    sync_token: Option<String>,
    sender: Sender<ClientMessage>,
) -> anyhow::Result<()> {
    let data_dir = Path::new("data");
    let session_file = data_dir.join("session");

    sync(client, sync_token, &session_file, sender)
        .await
        .map_err(Into::into)
}

async fn restore_session(session_file: &Path) -> anyhow::Result<(Client, Option<String>)> {
    info!(
        "Previous session found in '{}'",
        session_file.to_string_lossy()
    );

    let serialized_session = fs::read_to_string(session_file).await?;
    let FullSession {
        client_session,
        user_session,
        sync_token,
    } = serde_json::from_str(&serialized_session)?;

    let client = Client::builder()
        .homeserver_url(client_session.homeserver)
        .sqlite_store(client_session.db_path, Some(&client_session.passphrase))
        .build()
        .await?;

    info!("Restoring session for {}…", user_session.meta.user_id);

    client.restore_session(user_session).await?;

    Ok((client, sync_token))
}

async fn login(
    credentials: Credentials,
    data_dir: &Path,
    session_file: &Path,
) -> anyhow::Result<Client> {
    info!("No previous session found, logging in…");

    let (client, client_session) = build_client(&credentials, data_dir).await?;
    let matrix_auth = client.matrix_auth();

    match matrix_auth
        .login_username(&credentials.username, &credentials.password)
        .initial_device_display_name(env!("CARGO_PKG_NAME"))
        .await
    {
        Ok(_) => {
            info!("Logged in as {}", &credentials.username);
        }
        Err(error) => {
            info!("Error logging in: {}", error);
        }
    }

    let user_session = matrix_auth
        .session()
        .expect("A logged-in client should have a session");
    let serialized_session = serde_json::to_string(&FullSession {
        client_session,
        user_session,
        sync_token: None,
    })?;
    fs::write(session_file, serialized_session).await?;

    info!("Session persisted in {}", session_file.to_string_lossy());

    Ok(client)
}

async fn build_client(
    credentials: &Credentials,
    data_dir: &Path,
) -> anyhow::Result<(Client, ClientSession)> {
    let mut rng = StdRng::from_entropy();

    let db_subfolder: String = (&mut rng)
        .sample_iter(Alphanumeric)
        .take(7)
        .map(char::from)
        .collect();
    let db_path = data_dir.join(db_subfolder);

    let passphrase: String = (&mut rng)
        .sample_iter(Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    let homeserver = format!(
        "https://{}",
        credentials.username.split_once(':').unwrap().1
    );

    loop {
        match Client::builder()
            .homeserver_url(&homeserver)
            .sqlite_store(&db_path, Some(&passphrase))
            .build()
            .await
        {
            Ok(client) => {
                return Ok((
                    client,
                    ClientSession {
                        homeserver: homeserver.to_owned(),
                        db_path,
                        passphrase,
                    },
                ))
            }
            Err(error) => match &error {
                matrix_sdk::ClientBuildError::AutoDiscovery(_)
                | matrix_sdk::ClientBuildError::Url(_)
                | matrix_sdk::ClientBuildError::Http(_) => {
                    println!("Error checking the homeserver: {error}");
                    println!("Please try again\n");
                }
                _ => {
                    return Err(error.into());
                }
            },
        }
    }
}

async fn sync(
    client: Client,
    initial_sync_token: Option<String>,
    session_file: &Path,
    sender: Sender<ClientMessage>,
) -> anyhow::Result<()> {
    println!("Launching a first sync to ignore past messages…");

    let filter = FilterDefinition::with_lazy_loading();

    let mut sync_settings = SyncSettings::default().filter(filter.into());

    if let Some(sync_token) = initial_sync_token {
        sync_settings = sync_settings.token(sync_token);
    }

    loop {
        match client.sync_once(sync_settings.clone()).await {
            Ok(response) => {
                sync_settings = sync_settings.token(response.next_batch.clone());
                persist_sync_token(session_file, response.next_batch).await?;
                break;
            }
            Err(error) => {
                println!("An error occurred during initial sync: {error}");
                println!("Trying again…");
            }
        }
    }

    println!("The client is ready! Listening to new messages…");

    client.add_event_handler(move |event, room| {
        let sender = sender.clone();
        async move {
            on_room_message(event, room, sender).await;
        }
    });

    client
        .sync_with_result_callback(sync_settings, |sync_result| async move {
            let response = sync_result?;

            persist_sync_token(session_file, response.next_batch)
                .await
                .map_err(|err| Error::UnknownError(err.into()))?;

            Ok(LoopCtrl::Continue)
        })
        .await?;

    Ok(())
}

async fn persist_sync_token(session_file: &Path, sync_token: String) -> anyhow::Result<()> {
    let serialized_session = fs::read_to_string(session_file).await?;
    let mut full_session: FullSession = serde_json::from_str(&serialized_session)?;

    full_session.sync_token = Some(sync_token);
    let serialized_session = serde_json::to_string(&full_session)?;
    fs::write(session_file, serialized_session).await?;

    Ok(())
}

async fn on_room_message(
    event: OriginalSyncRoomMessageEvent,
    room: Room,
    sender: Sender<ClientMessage>,
) {
    if room.state() != RoomState::Joined {
        return;
    }
    if room.client().user_id().unwrap() == event.sender {
        return;
    }
    let MessageType::Text(text_content) = &event.content.msgtype else {
        return;
    };

    let room_name = match room.display_name().await {
        Ok(room_name) => room_name.to_string(),
        Err(error) => {
            println!("Error getting room display name: {error}");

            room.room_id().to_string()
        }
    };

    println!("[{room_name}] {}: {}", event.sender, text_content.body);

    let message = Message {
        sender: event.sender.to_string(),
        contents: text_content.body.clone(),
        timestamp: Local::now(),
    };

    if let Err(e) = sender.send(ClientMessage::NewMessage(message)) {
        println!("Error sending message to Iced application: {}", e);
    }
}
