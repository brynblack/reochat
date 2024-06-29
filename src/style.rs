use iced::{
    color,
    widget::{button, text_input},
    Background, Color, Theme,
};

pub(crate) struct ButtonComposerSend;

impl button::StyleSheet for ButtonComposerSend {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: iced::Background::Color(style.palette().primary).into(),
            border: iced::Border::with_radius(24.0),
            ..Default::default()
        }
    }

    fn hovered(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: iced::Background::Color(color!(0x004fee)).into(),
            border: iced::Border::with_radius(24.0),
            ..Default::default()
        }
    }
}

pub(crate) struct TextInputComposer;

impl text_input::StyleSheet for TextInputComposer {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            background: Background::Color(color!(0x4c4c4c)),
            border: iced::Border::with_radius(24.0),
            icon_color: Color::TRANSPARENT,
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

    fn disabled_color(&self, _style: &Self::Style) -> Color {
        color!(0x969696)
    }

    fn disabled(&self, _style: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            background: Background::Color(color!(0x4c4c4c)),
            border: iced::Border::with_radius(24.0),
            icon_color: Color::TRANSPARENT,
        }
    }
}
