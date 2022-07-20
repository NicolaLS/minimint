use minimint_api::Amount;
use minimint_core::modules::mint::tiered::coins::Coins;
use mint_client::mint::{CoinFinalizationData, SpendableCoin};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct InfoResponse {
    coins: Vec<CoinsByTier>,
    pending: PendingResponse,
}

// Holds quantity of coins per tier
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CoinsByTier {
    tier: u64,
    quantity: usize,
}

#[derive(Serialize)]
pub struct PendingResponse {
    transactions: usize,
    acc_qty_coins: usize,
    acc_val_amount: Amount,
}

impl InfoResponse {
    pub fn new(coins: Coins<SpendableCoin>, cfd: Vec<CoinFinalizationData>) -> Self {
        let info_coins: Vec<CoinsByTier> = coins
            .coins
            .iter()
            .map(|(tier, c)| CoinsByTier {
                quantity: c.len(),
                tier: tier.milli_sat,
            })
            .collect();
        Self {
            coins: info_coins,
            pending: PendingResponse::new(cfd),
        }
    }
}

impl PendingResponse {
    pub fn new(all_pending: Vec<CoinFinalizationData>) -> Self {
        let acc_qty_coins = all_pending.iter().map(|cfd| cfd.coin_count()).sum();
        let acc_val_amount = all_pending.iter().map(|cfd| cfd.coin_amount()).sum();
        Self {
            transactions: all_pending.len(),
            acc_qty_coins,
            acc_val_amount,
        }
    }
}
