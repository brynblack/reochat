use std::env;

use chrono::{DateTime, Local};
use iced::{
    alignment::Vertical,
    color, executor,
    theme::{self, Custom},
    widget::{scrollable, svg, Column, Container, Row, Scrollable, Text, TextInput},
    Application, Color, Command, Length, Padding, Theme,
};
use once_cell::sync::Lazy;

fn main() -> iced::Result {
    let username = env::args().nth(1).unwrap();
    let password = env::args().next().unwrap();

    Client::run(iced::Settings {
        antialiasing: true,
        flags: Flags { username, password },
        ..Default::default()
    })
}

#[derive(Default)]
struct Flags {
    username: String,
    password: String,
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
}

#[derive(Debug, Clone)]
enum ClientMessage {
    ComposerTyped(String),
    MessageSubmitted,
}

static SCROLLABLE_ID: Lazy<scrollable::Id> = Lazy::new(scrollable::Id::unique);

impl Application for Client {
    type Executor = executor::Default;
    type Message = ClientMessage;
    type Theme = Theme;
    type Flags = Flags;

    fn new(flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        (
            Self {
                username: flags.username,
                ..Default::default()
            },
            Command::none(),
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
        }
    }

    fn view(&self) -> iced::Element<'_, Self::Message, iced::Renderer<Self::Theme>> {
        let messages = Container::new(
            Scrollable::new(
                Column::with_children(
                    self.messages
                        .clone()
                        .into_iter()
                        .map(|msg| {
                            Column::new()
                                .push(
                                    Row::new()
                                        .push(Text::new(msg.sender))
                                        .push(
                                            Text::new(format!("{}", msg.timestamp.format("%H:%M")))
                                                .size(12),
                                        )
                                        .align_items(iced::Alignment::Center)
                                        .spacing(8),
                                )
                                .push(Text::new(msg.contents))
                                .into()
                        })
                        .collect(),
                )
                .spacing(8)
                .width(Length::Fill),
            )
            .id(SCROLLABLE_ID.clone()),
        )
        .align_y(Vertical::Bottom)
        .height(Length::Fill)
        .width(Length::Fill);

        let composer = Container::new(
            Row::new()
                .push(
                    TextInput::new("Message", &self.compose_value, ClientMessage::ComposerTyped)
                        .style(theme::TextInput::Custom(Box::new(style::TextInputComposer)))
                        .on_submit(ClientMessage::MessageSubmitted)
                        .padding(Padding {
                            top: 12.0,
                            right: 12.0,
                            bottom: 12.0,
                            left: 15.0,
                        }),
                )
                .push(
                    Container::new(
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
                    .style(theme::Container::Custom(Box::new(
                        style::ContainerComposerAddFile,
                    )))
                    .padding(Padding {
                        top: 12.0,
                        right: 10.0,
                        bottom: 12.0,
                        left: 14.0,
                    })
                    .center_x()
                    .center_y(),
                )
                .align_items(iced::Alignment::Center)
                .spacing(8),
        )
        .width(Length::Fill);

        let content = Column::new().push(messages).push(composer).spacing(16);

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(Vertical::Bottom)
            .padding(16)
            .into()
    }

    fn theme(&self) -> Self::Theme {
        Theme::Custom(Box::new(Custom::new(theme::Palette {
            background: Color::BLACK,
            text: Color::WHITE,
            primary: color!(0x2c6bee),
            success: Color::TRANSPARENT,
            danger: Color::TRANSPARENT,
        })))
    }
}

mod style {
    use iced::{
        color,
        widget::{container, text_input},
        Background, Color, Theme,
    };

    pub(crate) struct ContainerComposerAddFile;

    impl container::StyleSheet for ContainerComposerAddFile {
        type Style = Theme;

        fn appearance(&self, style: &Self::Style) -> container::Appearance {
            container::Appearance {
                background: style.palette().primary.into(),
                border_radius: 24.0,
                ..Default::default()
            }
        }
    }

    pub(crate) struct TextInputComposer;

    impl text_input::StyleSheet for TextInputComposer {
        type Style = Theme;

        fn active(&self, _style: &Self::Style) -> text_input::Appearance {
            text_input::Appearance {
                background: Background::Color(color!(0x2e2e2e)),
                border_radius: 24.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            }
        }

        fn focused(&self, _style: &Self::Style) -> text_input::Appearance {
            text_input::Appearance {
                ..self.active(&Theme::Dark)
            }
        }

        fn placeholder_color(&self, _style: &Self::Style) -> iced::Color {
            color!(0x969696)
        }

        fn value_color(&self, _style: &Self::Style) -> iced::Color {
            color!(0xffffff)
        }

        fn selection_color(&self, _style: &Self::Style) -> iced::Color {
            color!(0x0000ff)
        }
    }
}
