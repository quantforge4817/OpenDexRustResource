use iced::widget::{Column, Container, Row, Scrollable, Text};
use iced::Element;

use crate::gui_app::Message;
use crate::gui_model::TradeView;
use crate::gui_style::WARNING_COLOR;

fn state_color(state: &protocol::message::TradeState) -> iced::Color {
    match state {
        protocol::message::TradeState::Settled => iced::Color::from_rgb(0.2, 0.7, 0.3),
        protocol::message::TradeState::Cancelled
        | protocol::message::TradeState::Expired => iced::Color::from_rgb(0.6, 0.6, 0.6),
        protocol::message::TradeState::Disputed => iced::Color::from_rgb(0.9, 0.2, 0.2),
        _ => WARNING_COLOR,
    }
}

fn state_label(state: &protocol::message::TradeState) -> &'static str {
    match state {
        protocol::message::TradeState::Created => "Created",
        protocol::message::TradeState::Accepted => "Accepted",
        protocol::message::TradeState::EscrowFunding => "Escrow Funding",
        protocol::message::TradeState::EscrowFunded => "Escrow Funded",
        protocol::message::TradeState::PaymentPending => "Payment Pending",
        protocol::message::TradeState::PaymentSent => "Payment Sent",
        protocol::message::TradeState::PaymentConfirmed => "Payment Confirmed",
        protocol::message::TradeState::PayoutPending => "Payout Pending",
        protocol::message::TradeState::Settled => "Settled",
        protocol::message::TradeState::Cancelled => "Cancelled",
        protocol::message::TradeState::Expired => "Expired",
        protocol::message::TradeState::Disputed => "Disputed",
    }
}

fn trade_row(trade: &TradeView) -> Element<'_, Message> {
    let id_str = trade.id.to_string()[..8].to_string();
    let price_str = format!("{:.8}", trade.price).trim_end_matches('0').to_string();
    let amount_str = format!("{:.8}", trade.amount).trim_end_matches('0').to_string();
    let state_str = state_label(&trade.state);

    Row::new()
        .spacing(10)
        .push(Text::new(id_str))
        .push(Text::new(price_str))
        .push(Text::new(amount_str))
        .push(Text::new(state_str).width(iced::Length::Fill))
        .into()
}

pub fn trades_panel(trades: &[TradeView]) -> Element<'_, Message> {
    let header: Element<'_, Message> = Row::new()
        .spacing(10)
        .push(Text::new("ID"))
        .push(Text::new("Price"))
        .push(Text::new("Amount"))
        .push(Text::new("State").width(iced::Length::Fill))
        .into();

    let rows: Vec<Element<'_, Message>> = trades.iter().map(|t| trade_row(t)).collect();

    let content = Column::new()
        .spacing(5)
        .push(Text::new("Trades"))
        .push(header)
        .push(Column::with_children(rows));

    let scrollable = Scrollable::new(content).height(iced::Length::Fill);

    Container::new(scrollable)
        .padding(10)
        .into()
}
