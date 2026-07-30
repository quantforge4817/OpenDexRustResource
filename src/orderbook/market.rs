use serde::{Deserialize, Serialize};

use super::asset::Asset;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Market {
    pub base: Asset,
    pub quote: Asset,
}
