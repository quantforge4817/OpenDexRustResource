use iced::{Element, Length};
use iced::widget::{Container, Column, Row, Scrollable, Text, Button};
use crate::app::Message;
use crate::model::{GuiModel, TradeView};
use crate::style::{BG_PANEL, TEXT_SECONDARY, BID_COLOR, ASK_COLOR, WARNING_COLOR, ACCENT_COLOR};

pub fn trades_panel(model: &GuiModel) -> Element<'_, Message> {
    let trades = &model.trades;
    let header = Row::new()
        .spacing(6)
        .padding(iced::Padding::new(8.0).left(10).right(10))
        .push(Text::new("ID").size(11).width(Length::Fixed(50.0)).color(TEXT_SECONDARY))
        .push(Text::new("Market").size(11).width(Length::Fixed(50.0)).color(TEXT_SECONDARY))
        .push(Text::new("Price").size(11).width(Length::Fill).color(TEXT_SECONDARY))
        .push(Text::new("Amount").size(11).width(Length::Fill).color(TEXT_SECONDARY))
        .push(Text::new("State").size(11).width(Length::Fill).color(TEXT_SECONDARY))
        .push(Text::new("Links").size(11).width(Length::Fixed(40.0)).color(TEXT_SECONDARY));

    let list = trades.iter().fold(
        Column::new().spacing(0),
        |col, trade| {
            let id_short = trade.id.to_string()[..8].to_string();
            let state_color = match trade.state {
                protocol::message::TradeState::Settled => BID_COLOR,
                protocol::message::TradeState::Cancelled
                | protocol::message::TradeState::Expired => ASK_COLOR,
                protocol::message::TradeState::Disputed => WARNING_COLOR,
                _ => TEXT_SECONDARY,
            };
            
            // Show swap progress for active trades
            let swap_progress = match trade.state {
                protocol::message::TradeState::Created => "📋 Created",
                protocol::message::TradeState::Accepted => "✅ Accepted",
                protocol::message::TradeState::EscrowFunding => "🔒 Escrow Funding",
                protocol::message::TradeState::EscrowFunded => "🔒 Escrow Funded",
                protocol::message::TradeState::PaymentPending => "💰 Payment Pending",
                protocol::message::TradeState::PaymentSent => "📤 Payment Sent",
                protocol::message::TradeState::PaymentConfirmed => "✓ Payment Confirmed",
                protocol::message::TradeState::PayoutPending => "⏳ Payout Pending",
                protocol::message::TradeState::Settled => "✓ Settled",
                protocol::message::TradeState::Cancelled => "✗ Cancelled",
                protocol::message::TradeState::Expired => "⏰ Expired",
                protocol::message::TradeState::Disputed => "⚠ Disputed",
            };

            let row = Row::new()
                .spacing(6)
                .padding(iced::Padding::new(4.0).left(10).right(10))
                .push(Text::new(id_short).size(10).width(Length::Fixed(50.0)).color(TEXT_SECONDARY))
                .push(Text::new(&trade.market).size(10).width(Length::Fixed(50.0)))
                .push(Text::new(format!("{:.4}", trade.price)).size(10).width(Length::Fill))
                .push(Text::new(format!("{:.4}", trade.amount)).size(10).width(Length::Fill))
                .push(Text::new(swap_progress).size(9).width(Length::Fill).color(state_color))
                .push(Row::new()
                    .spacing(4)
                    .push(Button::new(Text::new("₿").size(9))
                        .on_press(Message::OpenBtcExplorer(trade.id))
                        .padding(iced::Padding::new(2.0)))
                    .push(Button::new(Text::new("ɱ").size(9))
                        .on_press(Message::OpenXmrExplorer(trade.id))
                        .padding(iced::Padding::new(2.0)))
                    .width(Length::Fixed(40.0))
                );
            col.push(row)
        }
    );

    let title = Container::new(Text::new("Trades").size(14))
        .padding(iced::Padding::new(8.0).left(10).bottom(4));

    let content = Column::new()
        .spacing(0)
        .push(title)
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
