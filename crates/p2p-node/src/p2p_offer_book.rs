/// P2P offer book for managing P2P trading offers.
///
/// Stores active P2P offers posted by the local node and received from peers.
/// Offers can be taken by other peers to initiate P2P trades.
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

use protocol::p2p::{P2POffer, P2PTrade};

use crate::persistence::{read, write};

const OFFERS_FILE: &str = "p2p_offers.json";
const TRADES_FILE: &str = "p2p_trades.json";

/// Storage for P2P offers and trades
#[derive(Debug, Clone)]
pub struct P2POfferBook {
    /// Active offers indexed by ID
    pub offers: HashMap<Uuid, P2POffer>,
    /// Active trades indexed by ID
    pub trades: HashMap<Uuid, P2PTrade>,
    /// Removed offer IDs (for sync)
    pub removed_offers: Vec<Uuid>,
}

impl P2POfferBook {
    /// Create a new, empty P2P offer book
    pub fn new() -> Self {
        Self {
            offers: HashMap::new(),
            trades: HashMap::new(),
            removed_offers: Vec::new(),
        }
    }

    /// Load from disk
    pub fn load() -> Self {
        let offers: HashMap<Uuid, P2POffer> = read(Path::new(OFFERS_FILE)).unwrap_or_default();
        let trades: HashMap<Uuid, P2PTrade> = read(Path::new(TRADES_FILE)).unwrap_or_default();
        Self {
            offers,
            trades,
            removed_offers: Vec::new(),
        }
    }

    /// Save offers to disk
    pub fn save_offers(&self) -> anyhow::Result<()> {
        write(Path::new(OFFERS_FILE), &self.offers)
    }

    /// Save trades to disk
    pub fn save_trades(&self) -> anyhow::Result<()> {
        write(Path::new(TRADES_FILE), &self.trades)
    }

    /// Add an offer to the book
    pub fn add_offer(&mut self, offer: P2POffer) {
        self.offers.insert(offer.id, offer);
    }

    /// Remove an offer from the book
    pub fn remove_offer(&mut self, offer_id: Uuid) -> Option<P2POffer> {
        let offer = self.offers.remove(&offer_id);
        if offer.is_some() {
            self.removed_offers.push(offer_id);
        }
        offer
    }

    /// Get an offer by ID
    pub fn get_offer(&self, offer_id: Uuid) -> Option<&P2POffer> {
        self.offers.get(&offer_id)
    }

    /// Add a trade to the book
    pub fn add_trade(&mut self, trade: P2PTrade) -> bool {
        if self.trades.contains_key(&trade.id) {
            return false;
        }
        self.trades.insert(trade.id, trade);
        true
    }

    /// Get a trade by ID
    pub fn get_trade(&self, trade_id: Uuid) -> Option<&P2PTrade> {
        self.trades.get(&trade_id)
    }

    /// Get a mutable reference to a trade
    pub fn get_trade_mut(&mut self, trade_id: Uuid) -> Option<&mut P2PTrade> {
        self.trades.get_mut(&trade_id)
    }

    /// Check if a trade belongs to a participant
    pub fn is_trade_participant(&self, trade: &P2PTrade, peer: &str) -> bool {
        trade.maker == peer || trade.taker == peer
    }

    /// Print all active offers (for debugging)
    pub fn print_offers(&self) {
        if self.offers.is_empty() {
            tracing::info!("=== P2P Offers (empty) ===");
            return;
        }

        tracing::info!("=== P2P Offers ({} active) ===", self.offers.len());
        for (id, offer) in &self.offers {
            tracing::info!(
                "  {} {} {} {} @ {} {} via {}",
                id,
                offer.side,
                offer.amount as f64 / protocol::message::AMOUNT_SCALE as f64,
                offer.crypto_asset,
                offer.price as f64 / protocol::p2p::FIAT_SCALE as f64,
                offer.currency,
                offer.payment_method,
            );
        }
    }

    /// Print all active trades (for debugging)
    pub fn print_trades(&self) {
        if self.trades.is_empty() {
            tracing::info!("=== P2P Trades (empty) ===");
            return;
        }

        tracing::info!("=== P2P Trades ({} active) ===", self.trades.len());
        for (id, trade) in &self.trades {
            tracing::info!(
                "  {} {} {} {} @ {} {} ({:?})",
                id,
                trade.side,
                trade.amount as f64 / protocol::message::AMOUNT_SCALE as f64,
                trade.crypto_asset,
                trade.price as f64 / protocol::p2p::FIAT_SCALE as f64,
                trade.currency,
                trade.state,
            );
        }
    }
}

impl Default for P2POfferBook {
    fn default() -> Self {
        Self::new()
    }
}
