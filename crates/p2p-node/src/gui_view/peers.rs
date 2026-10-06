use iced::widget::{button, Column, Container, Row, Scrollable, Text, TextInput};
use iced::Element;

use crate::gui_app::Message;
use crate::gui_model::GuiModel;
use crate::gui_style::ACCENT_COLOR;

pub fn peers_panel(model: &GuiModel) -> Element<'_, Message> {
    let peer_input = TextInput::new(
        "/ip4/.../tcp/.../p2p/...",
        &model.peer_input,
    )
    .on_input(Message::PeerInputChanged)
    .padding(5);

    let connect_button = button(Text::new("Connect"))
        .on_press(Message::ConnectPeer)
        .style(|_, _| iced::widget::button::Style {
            background: Some(iced::Background::Color(ACCENT_COLOR)),
            ..Default::default()
        });

    let input_row = Row::new()
        .spacing(5)
        .push(peer_input.width(iced::Length::Fill))
        .push(connect_button);

    let peer_rows: Vec<Element<'_, Message>> = model
        .peers
        .iter()
        .map(|peer| {
            let short_addr = if peer.address.len() > 20 {
                format!("{}...", &peer.address[..20])
            } else {
                peer.address.clone()
            };

            Row::new()
                .spacing(10)
                .push(Text::new("●"))
                .push(Text::new(short_addr))
                .into()
        })
        .collect();

    let content = Column::new()
        .spacing(10)
        .push(Text::new("Peers"))
        .push(input_row)
        .push(Scrollable::new(Column::with_children(peer_rows))
            .height(iced::Length::Fill));

    Container::new(content)
        .padding(10)
        .into()
}
