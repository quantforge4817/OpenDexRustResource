use iced::widget::{Column, Container, Row, Text};
use iced::Element;

use crate::gui_app::Message;
use crate::gui_model::{BalanceView, GuiModel};

fn asset_label(asset: &protocol::message::Asset) -> &'static str {
    match asset {
        protocol::message::Asset::BTC => "BTC",
        protocol::message::Asset::XMR => "XMR",
    }
}

fn balance_row(balance: &BalanceView) -> Element<'_, Message> {
    let label = asset_label(&balance.asset);
    let available = GuiModel::format_amount(balance.available);
    let locked = GuiModel::format_amount(balance.locked);
    let escrowed = GuiModel::format_amount(balance.escrowed);

    Row::new()
        .spacing(20)
        .push(Text::new(label))
        .push(Text::new(format!("Available: {}", available)))
        .push(Text::new(format!("Locked: {}", locked)))
        .push(Text::new(format!("Escrowed: {}", escrowed)))
        .into()
}

fn monero_wallet_status(model: &GuiModel) -> Element<'_, Message> {
    let status = if model.wallet_connected {
        let addr = model.wallet_address.as_deref().map(|a| {
            if a.len() > 30 {
                format!("{}...", &a[..30])
            } else {
                a.to_string()
            }
        });
        let balance = model.wallet_balance_available.map(|b| format!("{:.4} XMR", b));
        format!(
            "Monero wallet: connected | {} | {}",
            addr.unwrap_or_else(|| "no address".to_string()),
            balance.unwrap_or_else(|| "no balance".to_string())
        )
    } else {
        "Monero wallet: not connected".to_string()
    };

    Text::new(status).into()
}

fn btc_wallet_status(model: &GuiModel) -> Element<'_, Message> {
    let status = if model.btc_wallet_connected {
        let addr = model.btc_wallet_address.as_deref().map(|a| {
            if a.len() > 30 {
                format!("{}...", &a[..30])
            } else {
                a.to_string()
            }
        });
        let balance = model.btc_wallet_balance.map(|b| format!("{:.8} BTC", b));
        format!(
            "Bitcoin wallet: connected | {} | {}",
            addr.unwrap_or_else(|| "no address".to_string()),
            balance.unwrap_or_else(|| "no balance".to_string())
        )
    } else {
        "Bitcoin wallet: not connected".to_string()
    };

    Text::new(status).into()
}

pub fn wallet_panel(model: &GuiModel) -> Element<'_, Message> {
    let rows: Vec<Element<'_, Message>> = model.balances.iter().map(|b| balance_row(b)).collect();

    let content = Column::new()
        .spacing(5)
        .push(Text::new("Internal Balances:"))
        .push(Column::with_children(rows))
        .push(monero_wallet_status(model))
        .push(btc_wallet_status(model));

    Container::new(content)
        .padding(5)
        .into()
}
