//! PulsarTrack - Publisher Reputation (Soroban)
//! On-chain reputation scoring system for publishers on Stellar.

#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Address, Env,
};

#[contracttype]
#[derive(Clone)]
pub struct ReputationScore {
    pub publisher: Address,
    pub score: u32,          // 0-1000
    pub total_reviews: u64,
    pub positive_reviews: u64,
    pub negative_reviews: u64,
    pub slashes: u32,
    pub uptime_score: u32,   // 0-100
    pub quality_score: u32,  // 0-100
    pub last_updated: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct ReviewEntry {
    pub reviewer: Address,
    pub campaign_id: u64,
    pub positive: bool,
    pub rating: u32,  // 1-5
    pub timestamp: u64,
}

#[contracttype]
pub enum DataKey {
    Admin,
    ReputationOracle,
    Reputation(Address),
    Review(Address, u64),  // publisher, review_index
    ReviewCount(Address),
}

#[contract]
pub struct PublisherReputationContract;

#[contractimpl]
impl PublisherReputationContract {
    pub fn initialize(env: Env, admin: Address, oracle: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::ReputationOracle, &oracle);
    }

    pub fn init_publisher(env: Env, publisher: Address) {
        if env.storage().persistent().has(&DataKey::Reputation(publisher.clone())) {
            panic!("already initialized");
        }

        let score = ReputationScore {
            publisher: publisher.clone(),
            score: 500,
            total_reviews: 0,
            positive_reviews: 0,
            negative_reviews: 0,
            slashes: 0,
            uptime_score: 100,
            quality_score: 100,
            last_updated: env.ledger().timestamp(),
        };

        env.storage().persistent().set(&DataKey::Reputation(publisher), &score);
    }

    pub fn submit_review(
        env: Env,
        advertiser: Address,
        publisher: Address,
        campaign_id: u64,
        positive: bool,
        rating: u32,
    ) {
        advertiser.require_auth();

        if rating < 1 || rating > 5 {
            panic!("invalid rating");
        }

        let mut rep: ReputationScore = env
            .storage()
            .persistent()
            .get(&DataKey::Reputation(publisher.clone()))
            .expect("publisher not registered");

        let review = ReviewEntry {
            reviewer: advertiser,
            campaign_id,
            positive,
            rating,
            timestamp: env.ledger().timestamp(),
        };

        let count: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::ReviewCount(publisher.clone()))
            .unwrap_or(0);
        env.storage().persistent().set(&DataKey::Review(publisher.clone(), count), &review);
        env.storage().persistent().set(&DataKey::ReviewCount(publisher.clone()), &(count + 1));

        rep.total_reviews += 1;
        if positive {
            rep.positive_reviews += 1;
            // Increase score (max 1000)
            rep.score = (rep.score + rating as u32 * 2).min(1000);
        } else {
            rep.negative_reviews += 1;
            // Decrease score (min 0)
            rep.score = rep.score.saturating_sub(rating as u32 * 3);
        }
        rep.last_updated = env.ledger().timestamp();

        env.storage().persistent().set(&DataKey::Reputation(publisher), &rep);
    }

    pub fn slash_publisher(env: Env, oracle: Address, publisher: Address, penalty: u32) {
        oracle.require_auth();
        let stored_oracle: Address = env.storage().instance().get(&DataKey::ReputationOracle).unwrap();
        if oracle != stored_oracle {
            panic!("unauthorized");
        }

        let mut rep: ReputationScore = env
            .storage()
            .persistent()
            .get(&DataKey::Reputation(publisher.clone()))
            .expect("publisher not registered");

        rep.slashes += 1;
        rep.score = rep.score.saturating_sub(penalty);
        rep.last_updated = env.ledger().timestamp();

        env.storage().persistent().set(&DataKey::Reputation(publisher.clone()), &rep);

        env.events().publish(
            (symbol_short!("publisher"), symbol_short!("slashed")),
            (publisher, penalty),
        );
    }

    pub fn update_uptime(env: Env, oracle: Address, publisher: Address, uptime: u32) {
        oracle.require_auth();
        let stored_oracle: Address = env.storage().instance().get(&DataKey::ReputationOracle).unwrap();
        if oracle != stored_oracle {
            panic!("unauthorized");
        }

        if uptime > 100 {
            panic!("invalid uptime");
        }

        let mut rep: ReputationScore = env
            .storage()
            .persistent()
            .get(&DataKey::Reputation(publisher.clone()))
            .expect("publisher not registered");

        rep.uptime_score = uptime;
        // Recalculate score based on uptime
        let uptime_weight = uptime / 5; // up to 20 points
        rep.score = (rep.score + uptime_weight).min(1000);
        rep.last_updated = env.ledger().timestamp();

        env.storage().persistent().set(&DataKey::Reputation(publisher), &rep);
    }

    pub fn get_reputation(env: Env, publisher: Address) -> Option<ReputationScore> {
        env.storage().persistent().get(&DataKey::Reputation(publisher))
    }

    pub fn get_review(env: Env, publisher: Address, index: u64) -> Option<ReviewEntry> {
        env.storage().persistent().get(&DataKey::Review(publisher, index))
    }

    pub fn get_review_count(env: Env, publisher: Address) -> u64 {
        env.storage().persistent().get(&DataKey::ReviewCount(publisher)).unwrap_or(0)
    }
}

mod test;
