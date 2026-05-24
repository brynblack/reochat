use iced::{
    Background, Border, Color, Shadow, Theme, color,
    widget::{
        button, container,
        scrollable::{self, Scroller},
        text_input,
    },
};

fn rounded(radius: f32) -> Border {
    Border {
        radius: radius.into(),
        ..Border::default()
    }
}

pub(crate) fn button_room_item(_theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered => color!(0x004fee),
        _ => color!(0x4c4c4c),
    };
    button::Style {
        background: Some(Background::Color(background)),
        border: rounded(24.0),
        text_color: Color::WHITE,
        ..Default::default()
    }
}

pub(crate) fn button_composer_send(theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered => color!(0x004fee),
        _ => theme.palette().primary,
    };
    button::Style {
        background: Some(Background::Color(background)),
        border: rounded(24.0),
        ..Default::default()
    }
}

pub(crate) fn text_input_composer(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let value = match status {
        text_input::Status::Disabled => color!(0x969696),
        _ => color!(0xffffff),
    };
    text_input::Style {
        background: Background::Color(color!(0x4c4c4c)),
        border: rounded(24.0),
        icon: Color::TRANSPARENT,
        placeholder: color!(0x969696),
        value,
        selection: color!(0x0000ff),
    }
}

pub(crate) fn scrollable_room_list(
    _theme: &Theme,
    _status: scrollable::Status,
) -> scrollable::Style {
    let transparent_rail = || scrollable::Rail {
        background: Some(Background::Color(Color::TRANSPARENT)),
        border: Border::default(),
        scroller: Scroller {
            background: Background::Color(Color::TRANSPARENT),
            border: Border::default(),
        },
    };
    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: transparent_rail(),
        horizontal_rail: transparent_rail(),
        gap: None,
        auto_scroll: scrollable::AutoScroll {
            background: Background::Color(Color::TRANSPARENT),
            border: Border::default(),
            shadow: Shadow::default(),
            icon: Color::TRANSPARENT,
        },
    }
}
