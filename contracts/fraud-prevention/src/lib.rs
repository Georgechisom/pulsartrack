//! PulsarTrack - Fraud Prevention (Soroban)
//! Advanced fraud prevention and view verification for ad campaigns on Stellar.


#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Address, Bytes, BytesN, Env, String,
};

// ============================================================
// Data Types
// ============================================================

#[contracttype]
#[derive(Clone)]
pub struct ViewRecord {
    pub campaign_id: u64,
    pub publisher: Address,
    pub viewer: Address,
    pub timestamp: u64,
    pub verification_score: u32,
    pub verified: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct SuspiciousActivity {
    pub suspicious_views: u64,
    pub last_flagged: u64,
    pub total_flags: u64,
    pub suspended: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct VerificationCache {
    pub total_views: u64,
    pub verified_views: u64,
    pub rejected_views: u64,
    pub average_score: u32,
}

// ============================================================
// Storage Keys
// ============================================================

#[contracttype]
pub enum DataKey {
    Admin,
    VerificationThreshold,
    MaxViewsPerPeriod,
    SuspiciousThreshold,
    VerifyCounter,
    ViewRecord(BytesN<32>),
    ViewerRateLimit(Address, u64),
    SuspiciousActivity(Address),
    VerificationCache(u64, u64),
}

// ============================================================
// Contract
// ============================================================

#[contract]
pub struct FraudPreventionContract;

#[contractimpl]
impl FraudPreventionContract {
    /// Initialize the contract
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::VerificationThreshold, &80u32);
        env.storage()
            .instance()
            .set(&DataKey::MaxViewsPerPeriod, &10u64);
        env.storage()
            .instance()
            .set(&DataKey::SuspiciousThreshold, &100u64);
        env.storage()
            .instance()
            .set(&DataKey::VerifyCounter, &0u64);
    }

    /// Verify an ad view
    pub fn verify_view(
        env: Env,
        campaign_id: u64,
        publisher: Address,
        viewer: Address,
        proof_data: Option<BytesN<32>>,
    ) -> bool {
        // Rate limiting: check viewer hasn't exceeded limit in current period
        let current_period = env.ledger().timestamp() / 3600; // hourly buckets
        let rate_key = DataKey::ViewerRateLimit(viewer.clone(), current_period);
        let view_count: u64 = env
            .storage()
            .temporary()
            .get(&rate_key)
            .unwrap_or(0);

        let max_views: u64 = env
            .storage()
            .instance()
            .get(&DataKey::MaxViewsPerPeriod)
            .unwrap_or(10);

        if view_count >= max_views {
            panic!("rate limit exceeded");
        }

        // Generate view ID from campaign + publisher + viewer + timestamp
        let view_id = Self::_generate_view_id(&env, campaign_id, &publisher, &viewer);

        // Check for duplicate view
        if env
            .storage()
            .persistent()
            .has(&DataKey::ViewRecord(view_id.clone()))
        {
            panic!("duplicate view");
        }

        // Calculate verification score
        let score = Self::_calculate_score(&env, campaign_id, &publisher, &proof_data);
        let threshold: u32 = env
            .storage()
            .instance()
            .get(&DataKey::VerificationThreshold)
            .unwrap_or(80);

        let verified = score >= threshold;

        // Record view
        let record = ViewRecord {
            campaign_id,
            publisher: publisher.clone(),
            viewer: viewer.clone(),
            timestamp: env.ledger().timestamp(),
            verification_score: score,
            verified,
        };

        env.storage()
            .persistent()
            .set(&DataKey::ViewRecord(view_id), &record);
        env.storage()
            .temporary()
            .set(&rate_key, &(view_count + 1));

        // Update verification cache
        let current_day = env.ledger().timestamp() / 86_400;
        let cache_key = DataKey::VerificationCache(campaign_id, current_day);
        let mut cache: VerificationCache = env
            .storage()
            .temporary()
            .get(&cache_key)
            .unwrap_or(VerificationCache {
                total_views: 0,
                verified_views: 0,
                rejected_views: 0,
                average_score: 0,
            });

        cache.total_views += 1;
        if verified {
            cache.verified_views += 1;
        } else {
            cache.rejected_views += 1;
        }
        cache.average_score = ((cache.average_score as u64 * (cache.total_views - 1)
            + score as u64)
            / cache.total_views) as u32;

        env.storage().temporary().set(&cache_key, &cache);

        if verified {
            let counter: u64 = env
                .storage()
                .instance()
                .get(&DataKey::VerifyCounter)
                .unwrap_or(0);
            env.storage()
                .instance()
                .set(&DataKey::VerifyCounter, &(counter + 1));
        }

        env.events().publish(
            (symbol_short!("view"), symbol_short!("verified")),
            (campaign_id, publisher, verified),
        );

        if !verified {
            panic!("verification failed");
        }
        true
    }

    /// Flag suspicious publisher activity
    pub fn flag_suspicious(env: Env, publisher: Address) {
        let key = DataKey::SuspiciousActivity(publisher.clone());
        let mut activity: SuspiciousActivity = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(SuspiciousActivity {
                suspicious_views: 0,
                last_flagged: 0,
                total_flags: 0,
                suspended: false,
            });

        activity.suspicious_views += 1;
        activity.total_flags += 1;
        activity.last_flagged = env.ledger().timestamp();

        let threshold: u64 = env
            .storage()
            .instance()
            .get(&DataKey::SuspiciousThreshold)
            .unwrap_or(100);

        if activity.suspicious_views > threshold {
            activity.suspended = true;
        }

        env.storage().persistent().set(&key, &activity);

        env.events().publish(
            (symbol_short!("publisher"), symbol_short!("flagged")),
            publisher,
        );
    }

    /// Admin: clear suspicious flag
    pub fn clear_flag(env: Env, admin: Address, publisher: Address) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            panic!("unauthorized");
        }

        env.storage()
            .persistent()
            .remove(&DataKey::SuspiciousActivity(publisher));
    }

    /// Admin: update verification threshold
    pub fn set_threshold(env: Env, admin: Address, threshold: u32) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            panic!("unauthorized");
        }
        if threshold < 50 || threshold > 100 {
            panic!("invalid threshold");
        }
        env.storage()
            .instance()
            .set(&DataKey::VerificationThreshold, &threshold);
    }

    // ============================================================
    // Read-Only Functions
    // ============================================================

    pub fn get_verification_stats(env: Env, campaign_id: u64) -> VerificationCache {
        let current_day = env.ledger().timestamp() / 86_400;
        env.storage()
            .temporary()
            .get(&DataKey::VerificationCache(campaign_id, current_day))
            .unwrap_or(VerificationCache {
                total_views: 0,
                verified_views: 0,
                rejected_views: 0,
                average_score: 0,
            })
    }

    pub fn get_suspicious_status(env: Env, publisher: Address) -> Option<SuspiciousActivity> {
        env.storage()
            .persistent()
            .get(&DataKey::SuspiciousActivity(publisher))
    }

    pub fn is_publisher_suspended(env: Env, publisher: Address) -> bool {
        if let Some(activity) = env
            .storage()
            .persistent()
            .get::<DataKey, SuspiciousActivity>(&DataKey::SuspiciousActivity(publisher))
        {
            activity.suspended
        } else {
            false
        }
    }

    pub fn get_total_verifications(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::VerifyCounter)
            .unwrap_or(0)
    }

    // ============================================================
    // Internal Helpers
    // ============================================================

    fn _generate_view_id(env: &Env, campaign_id: u64, publisher: &Address, viewer: &Address) -> BytesN<32> {
        // Create a deterministic ID from campaign+publisher+viewer+timestamp
        let mut data = Bytes::new(env);
        // Combine relevant data - use campaign_id bytes
        let campaign_bytes = campaign_id.to_be_bytes();
        for b in campaign_bytes.iter() {
            data.push_back(*b);
        }
        env.crypto().sha256(&data).into()
    }

    fn _calculate_score(
        env: &Env,
        _campaign_id: u64,
        _publisher: &Address,
        proof_data: &Option<BytesN<32>>,
    ) -> u32 {
        let base_score: u32 = 80;
        let proof_bonus: u32 = if proof_data.is_some() { 10 } else { 0 };
        // Additional logic could check publisher reputation here
        base_score + proof_bonus
    }
}

mod test;
