//! PulsarTrack - Oracle Integration (Soroban)
//! Price feeds and external data oracle integration on Stellar.

#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Address, Env, String,
};

#[contracttype]
#[derive(Clone)]
pub struct PriceFeed {
    pub asset: String,
    pub price_usd: i128,   // price in USD * 1e7
    pub confidence: u32,   // 0-100
    pub timestamp: u64,
    pub source: String,
}

#[contracttype]
#[derive(Clone)]
pub struct PerformanceData {
    pub campaign_id: u64,
    pub impressions: u64,
    pub clicks: u64,
    pub conversions: u64,
    pub fraud_score: u32, // 0-100, lower is better
    pub timestamp: u64,
}

#[contracttype]
pub enum DataKey {
    Admin,
    PriceOracle,
    PerformanceOracle,
    PriceFeed(String),   // asset symbol
    PerformanceData(u64), // campaign_id
    OracleCount,
    AuthorizedOracle(Address),
}

#[contract]
pub struct OracleIntegrationContract;

#[contractimpl]
impl OracleIntegrationContract {
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::OracleCount, &0u32);
    }

    pub fn authorize_oracle(env: Env, admin: Address, oracle: Address) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            panic!("unauthorized");
        }
        env.storage().persistent().set(&DataKey::AuthorizedOracle(oracle.clone()), &true);

        let count: u32 = env.storage().instance().get(&DataKey::OracleCount).unwrap_or(0);
        env.storage().instance().set(&DataKey::OracleCount, &(count + 1));
    }

    pub fn revoke_oracle(env: Env, admin: Address, oracle: Address) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            panic!("unauthorized");
        }
        env.storage().persistent().remove(&DataKey::AuthorizedOracle(oracle));
    }

    pub fn update_price(
        env: Env,
        oracle: Address,
        asset: String,
        price_usd: i128,
        confidence: u32,
        source: String,
    ) {
        oracle.require_auth();
        Self::_require_oracle(&env, &oracle);

        let feed = PriceFeed {
            asset: asset.clone(),
            price_usd,
            confidence,
            timestamp: env.ledger().timestamp(),
            source,
        };

        env.storage().persistent().set(&DataKey::PriceFeed(asset.clone()), &feed);

        env.events().publish(
            (symbol_short!("oracle"), symbol_short!("price")),
            (asset, price_usd),
        );
    }

    pub fn update_performance(
        env: Env,
        oracle: Address,
        campaign_id: u64,
        impressions: u64,
        clicks: u64,
        conversions: u64,
        fraud_score: u32,
    ) {
        oracle.require_auth();
        Self::_require_oracle(&env, &oracle);

        let data = PerformanceData {
            campaign_id,
            impressions,
            clicks,
            conversions,
            fraud_score,
            timestamp: env.ledger().timestamp(),
        };

        env.storage().persistent().set(&DataKey::PerformanceData(campaign_id), &data);

        env.events().publish(
            (symbol_short!("oracle"), symbol_short!("perf")),
            campaign_id,
        );
    }

    pub fn get_price(env: Env, asset: String) -> Option<PriceFeed> {
        env.storage().persistent().get(&DataKey::PriceFeed(asset))
    }

    pub fn get_performance(env: Env, campaign_id: u64) -> Option<PerformanceData> {
        env.storage().persistent().get(&DataKey::PerformanceData(campaign_id))
    }

    pub fn is_oracle_authorized(env: Env, oracle: Address) -> bool {
        env.storage().persistent().get(&DataKey::AuthorizedOracle(oracle)).unwrap_or(false)
    }

    fn _require_oracle(env: &Env, oracle: &Address) {
        let is_auth: bool = env
            .storage()
            .persistent()
            .get(&DataKey::AuthorizedOracle(oracle.clone()))
            .unwrap_or(false);
        if !is_auth {
            panic!("not authorized oracle");
        }
    }
}

mod test;
