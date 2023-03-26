use chrono::{DateTime, Local};
use iced::{
    alignment::Vertical,
    executor,
    widget::{scrollable, Column, Container, Row, Scrollable, Text, TextInput},
    Application, Command, Length, Theme,
};
use once_cell::sync::Lazy;

fn main() -> iced::Result {
    Client::run(iced::Settings {
        antialiasing: true,
        ..Default::default()
    })
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
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        (
            Self {
                username: "Brynblack".into(),
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
                                            Text::new(format!(
                                                "{}",
                                                msg.timestamp.format("%H:%M:%S")
                                            ))
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
            TextInput::new(
                "Send a message...",
                &self.compose_value,
                ClientMessage::ComposerTyped,
            )
            .on_submit(ClientMessage::MessageSubmitted)
            .padding(8),
        )
        .width(Length::Fill);

        let content = Column::new().push(messages).push(composer).spacing(8);

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(Vertical::Bottom)
            .padding(8)
            .into()
    }

    fn theme(&self) -> Self::Theme {
        Theme::Dark
    }
}
