use iced::advanced::Hasher;
use matrix::Credentials;
use matrix_sdk::ruma::OwnedRoomId;
use std::{hash::Hash, str::FromStr, sync::Mutex};
mod matrix;
mod style;

use chrono::{DateTime, Local};
use clap::Parser;
use iced::{
    alignment::Vertical,
    color, executor,
    theme::{self, Custom},
    widget::{column, row, scrollable, svg, Button, Container, Scrollable, Text, TextInput},
    Application, Color, Command, Length, Padding, Theme,
};
use log::{info, warn};
use once_cell::sync::Lazy;
use std::{
    env,
    sync::{
        mpsc::{Receiver, Sender},
        Arc,
    },
};

#[derive(Default)]
struct Flags {
    username: String,
    password: String,
    roomid: String,
}

#[derive(Clone, Debug)]
struct Message {
    sender: String,
    contents: String,
    timestamp: DateTime<Local>,
}

#[derive(Default)]
struct Client {
    username: String,
    compose_value: String,
    messages: Vec<Message>,
    client: Option<matrix_sdk::Client>,
    sync_token: Option<String>,
    command_sender: Option<Sender<ClientMessage>>,
    command_receiver: Option<Arc<Mutex<Receiver<ClientMessage>>>>,
    roomid: String,
}

#[derive(Debug, Clone)]
enum ClientMessage {
    ComposerTyped(String),
    MessageSubmitted,
    LoggedIn(matrix_sdk::Client, Option<String>),
    FailedLogin,
    NewMessage(Message),
    None,
}

static SCROLLABLE_ID: Lazy<scrollable::Id> = Lazy::new(scrollable::Id::unique);

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Account username (e.g. `@meow123:matrix.org`)
    username: String,
    /// Account password
    password: String,
    /// Room ID to message in (WIP)
    roomid: String,
}

pub async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    Client::run(iced::Settings {
        antialiasing: true,
        flags: Flags {
            username: cli.username,
            password: cli.password,
            roomid: cli.roomid,
        },
        ..Default::default()
    })
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
}

impl Application for Client {
    type Executor = executor::Default;
    type Message = ClientMessage;
    type Theme = Theme;
    type Flags = Flags;

    fn new(flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        let (command_sender, command_receiver) = std::sync::mpsc::channel();

        let client = Self {
            username: flags.username.clone(),
            command_sender: Some(command_sender.clone()),
            command_receiver: Some(Arc::new(Mutex::new(command_receiver))),
            roomid: flags.roomid,
            ..Default::default()
        };

        let credentials = Credentials {
            username: flags.username,
            password: flags.password,
        };

        (
            client,
            Command::perform(matrix::run(credentials), |res| {
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

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        match message {
            ClientMessage::ComposerTyped(s) => {
                self.compose_value = s;
                Command::none()
            }
            ClientMessage::MessageSubmitted => match self.compose_value.as_str() {
                "" => Command::none(),
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
                        return Command::batch(vec![
                            scrollable::snap_to(
                                SCROLLABLE_ID.clone(),
                                scrollable::RelativeOffset::END,
                            ),
                            Command::perform(
                                async move {
                                    Client::send_message(client_clone, roomid, content)
                                        .await
                                        .unwrap();
                                },
                                |_| ClientMessage::None,
                            ),
                        ]);
                    };

                    scrollable::snap_to(SCROLLABLE_ID.clone(), scrollable::RelativeOffset::END)
                }
            },
            ClientMessage::LoggedIn(client, sync_token) => {
                self.client = Some(client.clone());
                self.sync_token = sync_token.clone();
                let command_sender = self.command_sender.clone().unwrap();
                Command::perform(
                    async move { matrix::start_event_loop(client, sync_token, command_sender).await },
                    |_| ClientMessage::FailedLogin,
                )
            }
            ClientMessage::NewMessage(message) => {
                self.messages.push(message);
                scrollable::snap_to(SCROLLABLE_ID.clone(), scrollable::RelativeOffset::END)
            }
            ClientMessage::FailedLogin => Command::none(),
            ClientMessage::None => Command::none(),
        }
    }

    fn view(&self) -> iced::Element<'_, Self::Message, Self::Theme, iced::Renderer> {
        let messages = Container::new(
            Scrollable::new(
                column(self.messages.clone().into_iter().map(|msg| {
                    column![
                        row![
                            Text::new(msg.sender),
                            Text::new(format!("{}", msg.timestamp.format("%H:%M"))).size(12)
                        ]
                        .align_items(iced::Alignment::Center)
                        .spacing(8),
                        Text::new(msg.contents)
                    ]
                    .into()
                }))
                .spacing(8)
                .padding(Padding::from([0, 20, 0, 0]))
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
                    .style(theme::TextInput::Custom(Box::new(style::TextInputComposer)))
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
                    .style(theme::Svg::custom_fn(|_theme| svg::Appearance {
                        color: Some(color!(0xffffff)),
                    })),
                )
                .padding(Padding {
                    top: 12.0,
                    right: 10.0,
                    bottom: 12.0,
                    left: 14.0,
                })
                .on_press(ClientMessage::MessageSubmitted)
                .style(theme::Button::Custom(Box::new(style::ButtonComposerSend))),
            ]
            .align_items(iced::Alignment::Center)
            .spacing(8),
        )
        .width(Length::Fill);

        let content = column![messages, composer].spacing(16);

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(Vertical::Bottom)
            .padding(16)
            .into()
    }

    fn theme(&self) -> Self::Theme {
        Theme::Custom(Arc::new(Custom::new(
            "default".to_string(),
            theme::Palette {
                background: Color::BLACK,
                text: Color::WHITE,
                primary: color!(0xffc0cb),
                success: Color::TRANSPARENT,
                danger: Color::TRANSPARENT,
            },
        )))
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        if let Some(receiver) = &self.command_receiver {
            iced::Subscription::from_recipe(PollMessages {
                receiver: Arc::clone(receiver),
            })
        } else {
            iced::Subscription::none()
        }
    }
}

struct PollMessages {
    receiver: Arc<Mutex<Receiver<ClientMessage>>>,
}

impl iced::advanced::subscription::Recipe for PollMessages {
    type Output = ClientMessage;

    fn hash(&self, state: &mut Hasher) {
        std::any::TypeId::of::<Self>().hash(state);
    }

    fn stream(
        self: Box<Self>,
        _input: iced::advanced::subscription::EventStream,
    ) -> iced::advanced::graphics::futures::BoxStream<Self::Output> {
        use iced::futures::StreamExt;

        let receiver = self.receiver.clone();

        let stream = iced::futures::stream::unfold(receiver, |receiver| async move {
            let message = {
                let receiver = receiver.lock().unwrap();
                receiver.recv().ok()
            };

            message.map(|msg| (msg, receiver))
        });

        stream.boxed()
    }
}
