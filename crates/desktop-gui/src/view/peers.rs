use iced::{Element, Length};
use iced::widget::{Button, Container, Column, Row, Scrollable, Text, TextInput};
use crate::app::Message;
use crate::model::GuiModel;
use crate::style::{BG_PANEL, TEXT_SECONDARY, ACCENT_COLOR};

pub fn peers_panel(model: &GuiModel) -> Element<'_, Message> {
    let peer_input = TextInput::new("Peer address (multiaddr)", &model.peer_input)
        .on_input(Message::PeerInputChanged)
        .padding(6);

    let connect_button = Button::new(Text::new("Connect").size(12))
        .on_press(Message::ConnectPeer)
        .padding(iced::Padding::new(10.0).top(6).bottom(6));

    let header = Row::new()
        .spacing(10)
        .padding(iced::Padding::new(8.0).left(10).right(10))
        .push(Text::new("Address").size(12).width(Length::Fill).color(TEXT_SECONDARY))
        .push(Text::new("Status").size(12).width(Length::Fixed(60.0)).color(TEXT_SECONDARY));

    let list = model.peers.iter().fold(
        Column::new().spacing(0),
        |col, peer| {
            let addr_short = if peer.address.len() > 40 {
                format!("...{}", &peer.address[peer.address.len() - 37..])
            } else {
                peer.address.clone()
            };
            let status = if peer.connected { "✓" } else { "✗" };
            col.push(Row::new()
                .spacing(10)
                .padding(iced::Padding::new(4.0).left(10).right(10))
                .push(Text::new(addr_short).size(10).width(Length::Fill).color(TEXT_SECONDARY))
                .push(Text::new(status).size(11).width(Length::Fixed(60.0)).color(ACCENT_COLOR)))
        }
    );

    let title = Container::new(Text::new("Peers").size(14))
        .padding(iced::Padding::new(8.0).left(10).bottom(4));

    let content = Column::new()
        .spacing(8)
        .push(title)
        .push(Row::new()
            .spacing(8)
            .padding(iced::Padding::new(8.0).left(10).right(10))
            .push(peer_input.width(Length::Fill))
            .push(connect_button))
        .push(header)
        .push(Scrollable::new(list).height(Length::Fill));

    Container::new(content)
        .padding(0)
        .height(Length::Fill)
        .style(|_theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(BG_PANEL)),
            ..Default::default()
        })
        .into()
}
