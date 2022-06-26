use anyhow::Result;
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::responses::RpcResult;

pub mod payload {
    use bitcoin::Transaction;
    use serde::{Deserialize, Serialize};

    use minimint_core::modules::wallet::txoproof::TxOutProof;

    #[derive(Deserialize, Serialize, Clone, Debug)]
    pub struct PeginPayload {
        pub txout_proof: TxOutProof,
        pub transaction: Transaction,
    }

    #[derive(Deserialize, Serialize, Clone, Debug)]
    pub struct PegoutPayload {
        pub address: bitcoin::Address,
        #[serde(with = "bitcoin::util::amount::serde::as_sat")]
        pub amount: bitcoin::Amount,
    }

    //TODO: remove this and also super::serde_invoice, when lightning_invoice "serde" feature becomes available
    #[derive(Deserialize, Serialize)]
    #[serde(transparent)]
    pub struct LnPayPayload {
        #[serde(with = "super::serde_invoice")]
        pub bolt11: lightning_invoice::Invoice,
    }
}

pub mod responses {
    use serde::{Deserialize, Serialize};

    use crate::CoinsByTier;
    use minimint_api::{Amount, OutPoint, TransactionId};
    use minimint_core::modules::mint::tiered::coins::Coins;
    use minimint_core::outcome::TransactionStatus;
    use mint_client::mint::{CoinFinalizationData, SpendableCoin};
    use mint_client::utils::serialize_coins;

    #[derive(Serialize, Deserialize)]
    pub enum RpcResult {
        Success(serde_json::Value),
        Failure(serde_json::Value),
    }
    #[derive(Serialize)]
    pub struct InfoResponse {
        coins: Vec<CoinsByTier>,
        pending: PendingResponse,
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

    #[derive(Deserialize, Serialize)]
    pub struct SpendResponse {
        pub coins: Coins<SpendableCoin>,
    }

    #[derive(Serialize)]
    pub struct EventsResponse {
        events: Vec<super::Event>,
    }

    #[derive(Serialize)]
    pub struct ReissueResponse {
        out_point: OutPoint,
        status: TransactionStatus,
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

        pub fn serialized(&self) -> String {
            serialize_coins(&self.coins)
        }
    }

    impl EventsResponse {
        pub fn new(events: Vec<super::Event>) -> Self {
            Self { events }
        }
    }

    impl ReissueResponse {
        pub fn new(out_point: OutPoint, status: TransactionStatus) -> Self {
            Self { out_point, status }
        }
    }
}

// Holds quantity of coins per tier
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CoinsByTier {
    tier: u64,
    quantity: usize,
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

pub async fn call<P: Serialize + ?Sized>(params: &P, enpoint: &str) -> Result<RpcResult> {
    let client = reqwest::Client::new();

    let response = client
        .post(format!("http://127.0.0.1:8081{}", enpoint))
        .json(params)
        .send()
        .await?;

    Ok(response.json().await?)
}

mod serde_invoice {
    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serializer};

    #[allow(missing_docs)]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<lightning_invoice::Invoice, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bolt11 = String::deserialize(deserializer)?
            .parse::<lightning_invoice::Invoice>()
            .map_err(|e| D::Error::custom(format!("{:?}", e)))?;

        Ok(bolt11)
    }
    #[allow(missing_docs)]
    #[allow(dead_code)]
    pub fn serialize<S>(
        invoice: &lightning_invoice::Invoice,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(invoice.to_string().as_str())
    }
}
