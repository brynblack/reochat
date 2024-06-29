mod matrix;
mod style;

use chrono::{DateTime, Local};
use iced::{
    alignment::Vertical,
    color, executor,
    theme::{self, Custom},
    widget::{column, row, scrollable, svg, Button, Container, Scrollable, Text, TextInput},
    Application, Color, Command, Length, Padding, Theme,
};
use log::warn;
use once_cell::sync::Lazy;
use std::{env, process, sync::Arc};

#[derive(Default)]
struct Flags {
    username: String,
    password: String,
    homeserver_url: String,
}

#[derive(Clone)]
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
    sync_token: String,
}

#[derive(Debug, Clone)]
enum ClientMessage {
    ComposerTyped(String),
    MessageSubmitted,
    LoggedIn(matrix_sdk::Client, String),
    FailedLogin,
}

static SCROLLABLE_ID: Lazy<scrollable::Id> = Lazy::new(scrollable::Id::unique);

pub async fn run() -> anyhow::Result<()> {
    let mut args = env::args();

    let cmd = args.next();
    let homeserver_url = args.next();
    let username = args.next();
    let password = args.next();

    let (homeserver_url, username, password) = match (homeserver_url, username, password) {
        (Some(a), Some(b), Some(c)) => (a, b, c),
        _ => {
            eprintln!(
                "Usage: {} <homeserver_url> <username> <password>",
                cmd.unwrap_or(env!("CARGO_PKG_NAME").to_string())
            );
            process::exit(1);
        }
    };

    Client::run(iced::Settings {
        antialiasing: true,
        flags: Flags {
            username,
            password,
            homeserver_url,
        },
        ..Default::default()
    })
    .map_err(anyhow::Error::from)
}

impl Application for Client {
    type Executor = executor::Default;
    type Message = ClientMessage;
    type Theme = Theme;
    type Flags = Flags;

    fn new(flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        (
            Self {
                username: flags.username.clone(),
                ..Default::default()
            },
            Command::perform(
                matrix::login(flags.homeserver_url, flags.username, flags.password),
                |res| {
                    let (client, token) = match res {
                        Ok((client, token)) => (client, token),
                        Err(err) => {
                            warn!("failed to login with error {}", err);
                            return ClientMessage::FailedLogin;
                        }
                    };
                    ClientMessage::LoggedIn(client, token)
                },
            ),
        )
    }

    fn title(&self) -> String {
        "ReoChat".into()
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

                    self.messages.push(message);
                    self.compose_value.clear();

                    scrollable::snap_to(SCROLLABLE_ID.clone(), scrollable::RelativeOffset::END)
                }
            },
            ClientMessage::LoggedIn(client, sync_token) => {
                self.client = Some(client);
                self.sync_token = sync_token;
                Command::none()
            }
            ClientMessage::FailedLogin => Command::none(),
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
                primary: color!(0x2c6bee),
                success: Color::TRANSPARENT,
                danger: Color::TRANSPARENT,
            },
        )))
    }
}
