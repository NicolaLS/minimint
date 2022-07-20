use bitcoin::Transaction;
use minimint_api::{Amount, TransactionId};
use minimint_core::modules::mint::tiered::coins::Coins;
use minimint_core::modules::wallet::txoproof::TxOutProof;
use mint_client::mint::{CoinFinalizationData, SpendableCoin};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Clone, Debug)]
pub struct PeginPayload {
    pub txout_proof: TxOutProof,
    pub transaction: Transaction,
}

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

#[derive(Serialize)]
pub struct PeginAddressResponse {
    pegin_address: bitcoin::Address,
}

#[derive(Serialize)]
pub struct PegInOutResponse {
    txid: TransactionId,
}

#[derive(Serialize)]
pub struct SpendResponse {
    pub coins: Coins<SpendableCoin>,
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

impl PeginAddressResponse {
    pub fn new(pegin_address: bitcoin::Address) -> Self {
        Self { pegin_address }
    }
}

impl PegInOutResponse {
    pub fn new(txid: TransactionId) -> Self {
        Self { txid }
    }
}

impl SpendResponse {
    pub fn new(coins: Coins<SpendableCoin>) -> Self {
        Self { coins }
    }
}
