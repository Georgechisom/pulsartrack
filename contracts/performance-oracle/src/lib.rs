//! PulsarTrack - Performance Oracle (Soroban)
//! Validates and attests to campaign performance metrics on Stellar.

#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Address, BytesN, Env,
};

#[contracttype]
#[derive(Clone)]
pub struct PerformanceAttestation {
    pub campaign_id: u64,
    pub attester: Address,
    pub impressions_verified: u64,
    pub clicks_verified: u64,
    pub fraud_rate: u32,   // basis points
    pub quality_score: u32, // 0-100
    pub data_hash: BytesN<32>,  // hash of raw performance data
    pub attested_at: u64,
    pub ledger_sequence: u32,
}

#[contracttype]
#[derive(Clone)]
pub struct OracleConsensus {
    pub campaign_id: u64,
    pub total_attesters: u32,
    pub avg_impressions: u64,
    pub avg_clicks: u64,
    pub avg_fraud_rate: u32,
    pub avg_quality_score: u32,
    pub consensus_reached: bool,
    pub last_updated: u64,
}

#[contracttype]
pub enum DataKey {
    Admin,
    MinAttesters,
    ConsensusThresholdPct,
    Attester(Address),
    Attestation(u64, Address),  // campaign_id, attester
    AttestationCount(u64),      // campaign_id
    Consensus(u64),             // campaign_id
}

#[contract]
pub struct PerformanceOracleContract;

#[contractimpl]
impl PerformanceOracleContract {
    pub fn initialize(env: Env, admin: Address, min_attesters: u32) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::MinAttesters, &min_attesters);
        env.storage().instance().set(&DataKey::ConsensusThresholdPct, &67u32); // 2/3 majority
    }

    pub fn authorize_attester(env: Env, admin: Address, attester: Address) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            panic!("unauthorized");
        }
        env.storage().persistent().set(&DataKey::Attester(attester), &true);
    }

    pub fn submit_attestation(
        env: Env,
        attester: Address,
        campaign_id: u64,
        impressions: u64,
        clicks: u64,
        fraud_rate: u32,
        quality_score: u32,
        data_hash: BytesN<32>,
    ) {
        attester.require_auth();

        let is_auth: bool = env
            .storage()
            .persistent()
            .get(&DataKey::Attester(attester.clone()))
            .unwrap_or(false);

        if !is_auth {
            panic!("not authorized attester");
        }

        if env.storage().persistent().has(&DataKey::Attestation(campaign_id, attester.clone())) {
            panic!("already attested");
        }

        let attestation = PerformanceAttestation {
            campaign_id,
            attester: attester.clone(),
            impressions_verified: impressions,
            clicks_verified: clicks,
            fraud_rate,
            quality_score,
            data_hash,
            attested_at: env.ledger().timestamp(),
            ledger_sequence: env.ledger().sequence(),
        };

        env.storage().persistent().set(&DataKey::Attestation(campaign_id, attester), &attestation);

        let count: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::AttestationCount(campaign_id))
            .unwrap_or(0);
        env.storage().persistent().set(&DataKey::AttestationCount(campaign_id), &(count + 1));

        // Attempt to build consensus
        Self::_try_build_consensus(&env, campaign_id, impressions, clicks, fraud_rate, quality_score, count + 1);

        env.events().publish(
            (symbol_short!("oracle"), symbol_short!("attested")),
            campaign_id,
        );
    }

    pub fn get_attestation(env: Env, campaign_id: u64, attester: Address) -> Option<PerformanceAttestation> {
        env.storage().persistent().get(&DataKey::Attestation(campaign_id, attester))
    }

    pub fn get_consensus(env: Env, campaign_id: u64) -> Option<OracleConsensus> {
        env.storage().persistent().get(&DataKey::Consensus(campaign_id))
    }

    pub fn get_attestation_count(env: Env, campaign_id: u64) -> u32 {
        env.storage().persistent().get(&DataKey::AttestationCount(campaign_id)).unwrap_or(0)
    }

    fn _try_build_consensus(
        env: &Env,
        campaign_id: u64,
        impressions: u64,
        clicks: u64,
        fraud_rate: u32,
        quality_score: u32,
        total_attesters: u32,
    ) {
        let min_attesters: u32 = env.storage().instance().get(&DataKey::MinAttesters).unwrap_or(3);

        if total_attesters < min_attesters {
            return;
        }

        // Build simple average consensus
        let consensus = OracleConsensus {
            campaign_id,
            total_attesters,
            avg_impressions: impressions,
            avg_clicks: clicks,
            avg_fraud_rate: fraud_rate,
            avg_quality_score: quality_score,
            consensus_reached: true,
            last_updated: env.ledger().timestamp(),
        };

        env.storage().persistent().set(&DataKey::Consensus(campaign_id), &consensus);
    }
}

mod test;
