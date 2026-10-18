use crate::app::Message;
use crate::model::GuiModel;
use crate::style::{ACCENT_COLOR, BG_PANEL, TEXT_SECONDARY};
use iced::widget::{Button, Column, Container, Row, Scrollable, Text, TextInput};
use iced::{Element, Length};

/// Well-known bootstrap nodes for new users to join the network
const BOOTSTRAP_NODES: &[(&str, &str)] = &[
    (
        "OpenDex Node 1",
        "/ip4/104.248.50.17/tcp/4001/p2p/12D3KooWBootstrap1",
    ),
    (
        "OpenDex Node 2",
        "/ip4/142.132.204.6/tcp/4001/p2p/12D3KooWBootstrap2",
    ),
    (
        "OpenDex Node 3",
        "/ip4/167.99.83.21/tcp/4001/p2p/12D3KooWBootstrap3",
    ),
];

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
        .push(
            Text::new("Address")
                .size(12)
                .width(Length::Fill)
                .color(TEXT_SECONDARY),
        )
        .push(
            Text::new("Status")
                .size(12)
                .width(Length::Fixed(60.0))
                .color(TEXT_SECONDARY),
        );

    let list = model
        .peers
        .iter()
        .fold(Column::new().spacing(0), |col, peer| {
            let addr_short = if peer.address.len() > 40 {
                format!("...{}", &peer.address[peer.address.len() - 37..])
            } else {
                peer.address.clone()
            };
            let status = if peer.connected { "✓" } else { "✗" };
            col.push(
                Row::new()
                    .spacing(10)
                    .padding(iced::Padding::new(4.0).left(10).right(10))
                    .push(
                        Text::new(addr_short)
                            .size(10)
                            .width(Length::Fill)
                            .color(TEXT_SECONDARY),
                    )
                    .push(
                        Text::new(status)
                            .size(11)
                            .width(Length::Fixed(60.0))
                            .color(ACCENT_COLOR),
                    ),
            )
        });

    let title = Container::new(Text::new("Peers").size(14))
        .padding(iced::Padding::new(8.0).left(10).bottom(4));

    // Bootstrap nodes section
    let bootstrap_title =
        Container::new(Text::new("⚡ Quick Connect").size(11).color(ACCENT_COLOR))
            .padding(iced::Padding::new(4.0).left(10).top(4));

    let bootstrap_list =
        BOOTSTRAP_NODES
            .iter()
            .fold(Column::new().spacing(2), |col, (name, addr)| {
                let btn = Button::new(
                    Row::new()
                        .spacing(6)
                        .push(Text::new(*name).size(10).color(TEXT_SECONDARY))
                        .push(Text::new("→").size(10).color(ACCENT_COLOR)),
                )
                .on_press(Message::ConnectBootstrap(addr.to_string()))
                .padding(iced::Padding::new(4.0).left(10).right(10));
                col.push(btn)
            });

    let bootstrap_section = Column::new()
        .spacing(2)
        .push(bootstrap_title)
        .push(bootstrap_list)
        .padding(iced::Padding::new(4.0).left(10).right(10));

    let content = Column::new()
        .spacing(4)
        .push(title)
        .push(bootstrap_section)
        .push(
            Row::new()
                .spacing(8)
                .padding(iced::Padding::new(8.0).left(10).right(10))
                .push(peer_input.width(Length::Fill))
                .push(connect_button),
        )
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
