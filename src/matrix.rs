use log::info;
use matrix_sdk::config::SyncSettings;

pub async fn login(
    homeserver_url: String,
    username: String,
    password: String,
) -> anyhow::Result<(matrix_sdk::Client, String)> {
    info!("connecting to homeserver {}", &homeserver_url);

    let client = matrix_sdk::Client::builder()
        .homeserver_url(homeserver_url)
        .build()
        .await?;

    client
        .login_username(&username, &password)
        .initial_device_display_name("ReoChat")
        .send()
        .await?;

    info!("logged in as {username}");

    let sync_token = client.sync_once(SyncSettings::default()).await?.next_batch;

    Ok((client, sync_token))
}
