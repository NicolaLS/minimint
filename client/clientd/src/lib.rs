use bitcoin::Transaction;
use minimint_api::{Amount, TransactionId};
use minimint_core::modules::mint::tiered::coins::Coins;
use minimint_core::modules::wallet::txoproof::TxOutProof;
use mint_client::mint::{CoinFinalizationData, SpendableCoin};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

#[derive(Deserialize, Clone, Debug)]
pub struct PeginPayload {
    pub txout_proof: TxOutProof,
    pub transaction: Transaction,
}

#[derive(Deserialize)]
#[serde(transparent)]
pub struct LnPayPayload {
    pub bolt11: lightning_invoice::Invoice,
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

#[derive(Serialize)]
pub struct EventsResponse {
    events: Vec<Event>,
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

impl EventsResponse {
    pub fn new(events: Vec<Event>) -> Self {
        Self { events }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Event {
    timestamp: u64,
    data: String,
}

impl Event {
    pub fn new(data: String) -> Self {
        let time = SystemTime::now();
        let d = time.duration_since(UNIX_EPOCH).unwrap();
        let timestamp = (d.as_secs() as u64) * 1000 + (u64::from(d.subsec_nanos()) / 1_000_000);
        Event { timestamp, data }
    }
}

pub struct EventLog {
    data: Mutex<VecDeque<Event>>,
}

impl EventLog {
    pub fn new(capacity: usize) -> Self {
        EventLog {
            data: Mutex::new(VecDeque::with_capacity(capacity)),
        }
    }
    pub async fn add(&self, data: String) -> u64 {
        let event = Event::new(data);
        self.add_event(event).await
    }
    pub async fn add_event(&self, event: Event) -> u64 {
        let mut events = self.data.lock().await;
        let timestamp = event.timestamp;

        if events.len() == events.capacity() {
            events.pop_front();
        }
        if let Some(last_event) = events.back() {
            // it is only needed to check the Order of the first one because this will be always done on 'add' so ( a,b,c,d) [d < e] => a,b,c also < e
            if event.timestamp < last_event.timestamp {
                let len = events.len();
                events.insert(len - 1, event)
            } else {
                events.push_back(event);
            }
        } else {
            events.push_back(event);
        }
        timestamp
    }
    pub async fn get(&self, timestamp: u64) -> Vec<Event> {
        let events = self.data.lock().await;
        let i = events
            .binary_search_by_key(&timestamp, |event| event.timestamp)
            .unwrap_or_else(|i| i);
        events.range(i..).cloned().collect()
    }
}
