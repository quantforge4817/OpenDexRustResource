use iced::widget::{Column, Row, Text, Container};
use iced::{Length, Element};
use crate::app::Message;
use crate::model::GuiModel;

/// Display the user's reputation stats.
pub fn reputation_panel<'a>(model: &'a GuiModel) -> Element<'a, Message> {
    let rep = model.reputation.clone();

    let tier_text = match rep.tier.as_str() {
        "Newcomer" => "🔵 Newcomer",
        "Trader" => "🟢 Trader",
        "Trusted" => "🟡 Trusted",
        "Verified" => "🟠 Verified",
        "Elite" => "🔴 Elite",
        _ => "🔵 Newcomer",
    };

    let panel = Column::new()
        .spacing(6)
        .padding(10)
        .push(
            Row::new()
                .spacing(10)
                .align_y(iced::Alignment::Center)
                .push(Text::new("Reputation").size(16))
                .push(Text::new(tier_text).size(14).width(Length::Fill))
        )
        .push(
            Row::new()
                .spacing(20)
                .push(Text::new(format!("Trust Score: {}/100", rep.trust_score)))
                .push(Text::new(format!("Response Rate: {:.0}%", rep.response_rate)))
        )
        .push(
            Row::new()
                .spacing(20)
                .push(Text::new(format!("Completed: {}", rep.completed_trades)))
                .push(Text::new(format!("Disputes: {}", rep.disputes)))
                .push(Text::new(format!("Failures: {}", rep.failures_to_respond)))
        )
        .push(
            Row::new()
                .spacing(20)
                .push(Text::new(format!("Makes: {}", rep.makes)))
                .push(Text::new(format!("Takes: {}", rep.takes)))
                .push(Text::new(format!("Rating: {:.1}/5 ({})", rep.average_rating, rep.rating_count)))
        )
        .push(
            Row::new()
                .spacing(20)
                .push(Text::new(format!("Reviews: {}", rep.review_count)))
        );

    Container::new(panel)
        .padding(10)
        .style(|_| iced::widget::container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb(0.05, 0.05, 0.05))),
            border: iced::Border {
                radius: 6.0.into(),
                width: 1.0,
                color: iced::Color::from_rgb(0.2, 0.2, 0.2),
            },
            ..Default::default()
        })
        .into()
}
