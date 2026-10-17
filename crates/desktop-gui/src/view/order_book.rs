use iced::{Element, Length};
use iced::widget::{Container, Column, Row, Scrollable, Text};
use crate::app::Message;
use crate::model::{GuiModel, OrderBookView};
use crate::style::{BG_PANEL, TEXT_SECONDARY, BID_COLOR, ASK_COLOR};

/// Reputation tier badge colors
fn tier_color(tier: &str) -> iced::Color {
    match tier {
        "Elite" => iced::Color::from([1.0, 0.84, 0.0, 1.0]),    // Gold
        "Verified" => iced::Color::from([0.0, 0.6, 1.0, 1.0]),  // Blue
        "Trusted" => iced::Color::from([0.3, 0.8, 0.3, 1.0]),   // Green
        "Trader" => iced::Color::from([0.6, 0.6, 0.6, 1.0]),    // Gray
        _ => iced::Color::from([0.4, 0.4, 0.4, 1.0]),           // Dark gray (Newcomer)
    }
}

fn tier_badge(tier: &str) -> String {
    match tier {
        "Elite" => "⭐".to_string(),
        "Verified" => "🔵".to_string(),
        "Trusted" => "🟢".to_string(),
        "Trader" => "⚪".to_string(),
        _ => "⚫".to_string(),
    }
}

pub fn order_book_panel<'a>(model: &'a GuiModel, order_book: &'a OrderBookView) -> Element<'a, Message> {
    let header = Row::new()
        .spacing(6)
        .padding(iced::Padding::new(8.0).left(10).right(10))
        .push(Text::new("Price").size(12).width(Length::Fixed(80.0)).color(TEXT_SECONDARY))
        .push(Text::new("Amount").size(12).width(Length::Fixed(70.0)).color(TEXT_SECONDARY))
        .push(Text::new("Trader").size(12).width(Length::Fixed(60.0)).color(TEXT_SECONDARY))
        .push(Text::new("Trust").size(12).width(Length::Fixed(40.0)).color(TEXT_SECONDARY))
        .push(Text::new("ID").size(12).width(Length::Fill).color(TEXT_SECONDARY));

    let asks_col = order_book.asks.iter().rev().fold(
        Column::new().spacing(0),
        |col, order| {
            let price = GuiModel::format_price(order.price);
            let amount = GuiModel::format_amount(order.amount);
            let id_short = order.id.to_string()[..8].to_string();
            let trader_short = if order.trader.len() > 8 {
                format!("{}...", &order.trader[..6])
            } else {
                order.trader.clone()
            };
            // Look up reputation from auth service
            let (trust_text, tier_text, trust_color) = {
                if let Ok(auth) = model.auth_service.lock() {
                    if let Some(user) = auth.get_user_by_username(&order.trader) {
                        let score = user.reputation.trust_score(0);
                        let tier = user.reputation.tier(0);
                        let badge = tier_badge(&tier.to_string());
                        (format!("{}", score), badge, tier_color(&tier.to_string()))
                    } else {
                        ("?".to_string(), "⚫".to_string(), iced::Color::from([0.4, 0.4, 0.4, 1.0]))
                    }
                } else {
                    ("?".to_string(), "⚫".to_string(), iced::Color::from([0.4, 0.4, 0.4, 1.0]))
                }
            };
            col.push(Row::new()
                .spacing(6)
                .padding(iced::Padding::new(4.0).left(10).right(10))
                .push(Text::new(price).size(11).width(Length::Fixed(80.0)).color(ASK_COLOR))
                .push(Text::new(amount).size(11).width(Length::Fixed(70.0)))
                .push(Text::new(trader_short).size(10).width(Length::Fixed(60.0)).color(TEXT_SECONDARY))
                .push(Row::new().spacing(2)
                    .push(Text::new(trust_text).size(10).color(trust_color))
                    .push(Text::new(tier_text).size(10)))
                .push(Text::new(id_short).size(10).width(Length::Fill).color(TEXT_SECONDARY)))
        }
    );

    let bids_col = order_book.bids.iter().fold(
        Column::new().spacing(0),
        |col, order| {
            let price = GuiModel::format_price(order.price);
            let amount = GuiModel::format_amount(order.amount);
            let id_short = order.id.to_string()[..8].to_string();
            let trader_short = if order.trader.len() > 8 {
                format!("{}...", &order.trader[..6])
            } else {
                order.trader.clone()
            };
            let (trust_text, tier_text, trust_color) = {
                if let Ok(auth) = model.auth_service.lock() {
                    if let Some(user) = auth.get_user_by_username(&order.trader) {
                        let score = user.reputation.trust_score(0);
                        let tier = user.reputation.tier(0);
                        let badge = tier_badge(&tier.to_string());
                        (format!("{}", score), badge, tier_color(&tier.to_string()))
                    } else {
                        ("?".to_string(), "⚫".to_string(), iced::Color::from([0.4, 0.4, 0.4, 1.0]))
                    }
                } else {
                    ("?".to_string(), "⚫".to_string(), iced::Color::from([0.4, 0.4, 0.4, 1.0]))
                }
            };
            col.push(Row::new()
                .spacing(6)
                .padding(iced::Padding::new(4.0).left(10).right(10))
                .push(Text::new(price).size(11).width(Length::Fixed(80.0)).color(BID_COLOR))
                .push(Text::new(amount).size(11).width(Length::Fixed(70.0)))
                .push(Text::new(trader_short).size(10).width(Length::Fixed(60.0)).color(TEXT_SECONDARY))
                .push(Row::new().spacing(2)
                    .push(Text::new(trust_text).size(10).color(trust_color))
                    .push(Text::new(tier_text).size(10)))
                .push(Text::new(id_short).size(10).width(Length::Fill).color(TEXT_SECONDARY)))
        }
    );

    let separator = Row::new()
        .padding(iced::Padding::new(4.0).left(10).right(10))
        .push(Text::new("—").size(11).width(Length::Fill).color(TEXT_SECONDARY));

    let all_rows = Column::new()
        .spacing(0)
        .push(asks_col)
        .push(separator)
        .push(bids_col);

    let title = Container::new(Text::new("Order Book").size(14))
        .padding(iced::Padding::new(8.0).left(10).bottom(4));

    let content = Column::new()
        .spacing(0)
        .push(title)
        .push(header)
        .push(Scrollable::new(all_rows).height(Length::Fill));

    Container::new(content)
        .padding(0)
        .height(Length::Fill)
        .style(|_theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(BG_PANEL)),
            ..Default::default()
        })
        .into()
}

/// Tooltip for reputation info
pub fn reputation_tooltip(trader: &str, model: &GuiModel) -> Option<String> {
    if let Ok(auth) = model.auth_service.lock() {
        if let Some(user) = auth.get_user_by_username(trader) {
            let rep = &user.reputation;
            return Some(format!(
                "{} | Trust: {} | {} | {} trades | {} reviews",
                trader,
                rep.trust_score(0),
                rep.tier(0),
                rep.completed_trades,
                rep.review_count()
            ));
        }
    }
    None
}
