use matrix::Credentials;
use matrix_sdk::ruma::OwnedRoomId;
use std::{
    str::FromStr,
    sync::{LazyLock, OnceLock},
};
use tokio::sync::{
    Mutex,
    mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
};
mod matrix;
mod style;

use chrono::{DateTime, Local};
use clap::Parser;
use iced::{
    Color, Length, Padding, Task, Theme,
    alignment::Vertical,
    color,
    theme::{self, Custom},
    widget::{
        Button, Container, Id, Scrollable, Text, TextInput, column, operation, row, scrollable, svg,
    },
};
use log::{info, warn};
use std::{env, sync::Arc};

#[derive(Clone, Debug)]
struct Message {
    sender: String,
    contents: String,
    timestamp: DateTime<Local>,
}

struct Client {
    username: String,
    compose_value: String,
    messages: Vec<Message>,
    client: Option<matrix_sdk::Client>,
    sync_token: Option<String>,
    command_sender: Option<UnboundedSender<ClientMessage>>,
    roomid: String,
}

impl Drop for Client {
    fn drop(&mut self) {
        if let Some(client) = self.client.take() {
            std::mem::forget(client);
        }
    }
}

static MATRIX_RECEIVER: OnceLock<Arc<Mutex<UnboundedReceiver<ClientMessage>>>> = OnceLock::new();

#[derive(Debug, Clone)]
enum ClientMessage {
    ComposerTyped(String),
    MessageSubmitted,
    LoggedIn(matrix_sdk::Client, Option<String>),
    FailedLogin,
    NewMessage(Message),
    RoomChanged(OwnedRoomId),
    None,
}

static SCROLLABLE_ID: LazyLock<Id> = LazyLock::new(Id::unique);

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Account username (e.g. `@meow123:matrix.org`)
    #[arg(env = "REOCHAT_USERNAME")]
    username: String,
    /// Account password
    #[arg(env = "REOCHAT_PASSWORD", hide_env_values = true)]
    password: String,
}

pub fn run() -> anyhow::Result<()> {
    iced::application(Client::new, Client::update, Client::view)
        .title(Client::title)
        .theme(Client::theme)
        .subscription(Client::subscription)
        .antialiasing(true)
        .run()
        .map_err(anyhow::Error::from)
}

impl Client {
    async fn send_message(
        client: matrix_sdk::Client,
        roomid: String,
        content: String,
    ) -> Result<(), matrix_sdk::Error> {
        let content =
            matrix_sdk::ruma::events::room::message::RoomMessageEventContent::text_plain(content);
        client
            .get_room(&OwnedRoomId::from_str(&roomid).unwrap())
            .unwrap()
            .send(content)
            .await?;
        Ok(())
    }

    fn new() -> (Self, Task<ClientMessage>) {
        let cli = Cli::parse();

        let (command_sender, command_receiver) = unbounded_channel();

        let _ = MATRIX_RECEIVER.set(Arc::new(Mutex::new(command_receiver)));

        let client = Self {
            username: cli.username.clone(),
            compose_value: String::new(),
            messages: Vec::new(),
            client: None,
            sync_token: None,
            command_sender: Some(command_sender.clone()),
            roomid: String::new(),
        };

        let credentials = Credentials {
            username: cli.username,
            password: cli.password,
        };

        (
            client,
            Task::perform(matrix::run(credentials), |res| {
                let (client, token) = match res {
                    Ok((client, token)) => (client, token),
                    Err(err) => {
                        warn!("Failed to login with error {}", err);
                        return ClientMessage::FailedLogin;
                    }
                };
                info!("Logged in as {}", client.user_id().unwrap());
                ClientMessage::LoggedIn(client, token)
            }),
        )
    }

    fn title(&self) -> String {
        env!("CARGO_PKG_NAME").into()
    }

    fn update(&mut self, message: ClientMessage) -> Task<ClientMessage> {
        match message {
            ClientMessage::ComposerTyped(s) => {
                self.compose_value = s;
                Task::none()
            }
            ClientMessage::MessageSubmitted => match self.compose_value.as_str() {
                "" => Task::none(),
                _ => {
                    let message = Message {
                        sender: self.username.clone(),
                        contents: self.compose_value.clone(),
                        timestamp: Local::now(),
                    };

                    self.messages.push(message.clone());
                    self.compose_value.clear();

                    if let Some(client) = &self.client {
                        let client_clone = client.clone();
                        let roomid = self.roomid.clone();
                        let content = message.contents.clone();
                        return Task::batch(vec![
                            operation::snap_to(
                                SCROLLABLE_ID.clone(),
                                scrollable::RelativeOffset::END,
                            ),
                            Task::perform(
                                async move {
                                    Client::send_message(client_clone, roomid, content)
                                        .await
                                        .unwrap();
                                },
                                |_| ClientMessage::None,
                            ),
                        ]);
                    };

                    operation::snap_to(SCROLLABLE_ID.clone(), scrollable::RelativeOffset::END)
                }
            },
            ClientMessage::LoggedIn(client, sync_token) => {
                self.client = Some(client.clone());
                self.sync_token = sync_token.clone();
                let command_sender = self.command_sender.clone().unwrap();
                Task::perform(
                    async move { matrix::start_event_loop(client, sync_token, command_sender).await },
                    |_| ClientMessage::FailedLogin,
                )
            }
            ClientMessage::NewMessage(message) => {
                self.messages.push(message);
                operation::snap_to(SCROLLABLE_ID.clone(), scrollable::RelativeOffset::END)
            }
            ClientMessage::RoomChanged(roomid) => {
                self.roomid = roomid.to_string();
                Task::none()
            }
            ClientMessage::FailedLogin => Task::none(),
            ClientMessage::None => Task::none(),
        }
    }

    fn view(&self) -> iced::Element<'_, ClientMessage, Theme, iced::Renderer> {
        let infobar = row![Text::new(
            self.client
                .clone()
                .and_then(|client| {
                    let binding = client.rooms();
                    let room = binding
                        .iter()
                        .find(|room| room.room_id().as_str() == self.roomid)?;

                    let out = room.name().unwrap_or_else(|| {
                        room.direct_targets()
                            .iter()
                            .map(|id| id.to_string())
                            .collect::<Vec<String>>()
                            .join(", ")
                    });

                    Some(out)
                })
                .unwrap_or_default()
        )];

        let timeline = Container::new(
            Scrollable::new(
                column(self.messages.clone().into_iter().map(|msg| {
                    column![
                        row![
                            Text::new(msg.sender),
                            Text::new(format!("{}", msg.timestamp.format("%H:%M"))).size(12)
                        ]
                        .align_y(iced::Alignment::Center)
                        .spacing(8),
                        Text::new(msg.contents)
                    ]
                    .into()
                }))
                .spacing(8)
                .padding(Padding {
                    top: 0.0,
                    right: 20.0,
                    bottom: 0.0,
                    left: 0.0,
                })
                .width(Length::Fill),
            )
            .id(SCROLLABLE_ID.clone()),
        )
        .align_y(Vertical::Bottom)
        .height(Length::Fill)
        .width(Length::Fill);

        let composer = Container::new(
            row![
                TextInput::new("Message", &self.compose_value)
                    .on_input(ClientMessage::ComposerTyped)
                    .style(style::text_input_composer)
                    .on_submit(ClientMessage::MessageSubmitted)
                    .padding(Padding {
                        top: 12.0,
                        right: 12.0,
                        bottom: 12.0,
                        left: 15.0,
                    }),
                Button::new(
                    svg::Svg::from_path(format!(
                        "{}/resources/send.svg",
                        env!("CARGO_MANIFEST_DIR"),
                    ))
                    .width(20)
                    .height(20)
                    .style(|_theme, _status| svg::Style {
                        color: Some(color!(0xffffff)),
                    }),
                )
                .padding(Padding {
                    top: 12.0,
                    right: 10.0,
                    bottom: 12.0,
                    left: 14.0,
                })
                .on_press(ClientMessage::MessageSubmitted)
                .style(style::button_composer_send),
            ]
            .align_y(iced::Alignment::Center)
            .spacing(8),
        )
        .width(Length::Fill);

        let room = column![infobar, timeline, composer].spacing(16);

        let room_list: Vec<
            iced::advanced::graphics::core::Element<'_, ClientMessage, Theme, iced::Renderer>,
        > = match &self.client {
            Some(client) => client
                .rooms()
                .into_iter()
                .map(|room| {
                    Button::new(Text::new(room.name().unwrap_or_else(|| {
                        room.direct_targets()
                            .iter()
                            .map(|id| id.to_string())
                            .collect::<Vec<String>>()
                            .join(", ")
                    })))
                    .style(style::button_room_item)
                    .on_press(ClientMessage::RoomChanged(room.room_id().into()))
                    .into()
                })
                .collect(),
            None => vec![],
        };

        let rooms = Scrollable::new(column(room_list).spacing(16))
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(0).scroller_width(0),
            ))
            .style(style::scrollable_room_list);

        let content = row![rooms, room].spacing(16);

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(Vertical::Bottom)
            .padding(16)
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::Custom(Arc::new(Custom::new(
            "default".to_string(),
            theme::Palette {
                background: color!(0x1E1E2E),
                text: Color::WHITE,
                primary: color!(0xffc0cb),
                success: Color::TRANSPARENT,
                danger: Color::TRANSPARENT,
                warning: color!(0xff0000),
            },
        )))
    }

    fn subscription(&self) -> iced::Subscription<ClientMessage> {
        if MATRIX_RECEIVER.get().is_some() {
            iced::Subscription::run(matrix_event_stream)
        } else {
            iced::Subscription::none()
        }
    }
}

fn matrix_event_stream() -> impl iced::futures::Stream<Item = ClientMessage> {
    let receiver = MATRIX_RECEIVER
        .get()
        .expect("matrix receiver not initialized")
        .clone();

    iced::futures::stream::unfold(receiver, |receiver| async move {
        let message = receiver.lock().await.recv().await;
        message.map(|msg| (msg, receiver))
    })
}
