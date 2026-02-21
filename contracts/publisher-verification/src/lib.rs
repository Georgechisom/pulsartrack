//! PulsarTrack - Publisher Verification (Soroban)
//! Publisher registration, KYC, and verification on Stellar.


#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Address, Env, String,
};

// ============================================================
// Data Types
// ============================================================

#[contracttype]
#[derive(Clone, PartialEq)]
pub enum VerificationStatus {
    Pending,
    Verified,
    Rejected,
    Suspended,
    Revoked,
}

#[contracttype]
#[derive(Clone)]
pub enum PublisherTier {
    Bronze,
    Silver,
    Gold,
    Platinum,
}

#[contracttype]
#[derive(Clone)]
pub struct Publisher {
    pub owner: Address,
    pub status: VerificationStatus,
    pub tier: PublisherTier,
    pub domain: String,
    pub reputation_score: u32,
    pub total_earnings: i128,
    pub total_impressions: u64,
    pub join_ledger: u32,
    pub verified_at: Option<u64>,
    pub last_active: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct KycRecord {
    pub publisher: Address,
    pub kyc_hash: String,     // hash of KYC documents stored off-chain
    pub kyc_provider: String, // name of KYC provider
    pub verified: bool,
    pub submitted_at: u64,
    pub verified_at: Option<u64>,
}

// ============================================================
// Storage Keys
// ============================================================

#[contracttype]
pub enum DataKey {
    Admin,
    PublisherCount,
    Publisher(Address),
    KycRecord(Address),
    DomainOwner(String),
}

// ============================================================
// Contract
// ============================================================

#[contract]
pub struct PublisherVerificationContract;

#[contractimpl]
impl PublisherVerificationContract {
    /// Initialize the contract
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::PublisherCount, &0u64);
    }

    /// Register as a publisher (self-registration)
    pub fn register_publisher(env: Env, publisher: Address, domain: String) {
        publisher.require_auth();

        if env
            .storage()
            .persistent()
            .has(&DataKey::Publisher(publisher.clone()))
        {
            panic!("already registered");
        }

        // Check domain not taken
        if env
            .storage()
            .persistent()
            .has(&DataKey::DomainOwner(domain.clone()))
        {
            panic!("domain already registered");
        }

        let pub_data = Publisher {
            owner: publisher.clone(),
            status: VerificationStatus::Pending,
            tier: PublisherTier::Bronze,
            domain: domain.clone(),
            reputation_score: 0,
            total_earnings: 0,
            total_impressions: 0,
            join_ledger: env.ledger().sequence(),
            verified_at: None,
            last_active: env.ledger().timestamp(),
        };

        env.storage()
            .persistent()
            .set(&DataKey::Publisher(publisher.clone()), &pub_data);
        env.storage()
            .persistent()
            .set(&DataKey::DomainOwner(domain), &publisher);

        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PublisherCount)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::PublisherCount, &(count + 1));

        env.events().publish(
            (symbol_short!("publisher"), symbol_short!("register")),
            publisher,
        );
    }

    /// Submit KYC documents (publisher)
    pub fn submit_kyc(
        env: Env,
        publisher: Address,
        kyc_hash: String,
        kyc_provider: String,
    ) {
        publisher.require_auth();

        if !env
            .storage()
            .persistent()
            .has(&DataKey::Publisher(publisher.clone()))
        {
            panic!("not registered");
        }

        let kyc = KycRecord {
            publisher: publisher.clone(),
            kyc_hash,
            kyc_provider,
            verified: false,
            submitted_at: env.ledger().timestamp(),
            verified_at: None,
        };

        env.storage()
            .persistent()
            .set(&DataKey::KycRecord(publisher.clone()), &kyc);

        env.events().publish(
            (symbol_short!("kyc"), symbol_short!("submitted")),
            publisher,
        );
    }

    /// Verify a publisher (admin only)
    pub fn verify_publisher(
        env: Env,
        admin: Address,
        publisher: Address,
        initial_tier: PublisherTier,
    ) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            panic!("unauthorized");
        }

        let mut pub_data: Publisher = env
            .storage()
            .persistent()
            .get(&DataKey::Publisher(publisher.clone()))
            .expect("publisher not found");

        pub_data.status = VerificationStatus::Verified;
        pub_data.tier = initial_tier;
        pub_data.verified_at = Some(env.ledger().timestamp());
        pub_data.reputation_score = 100;

        env.storage()
            .persistent()
            .set(&DataKey::Publisher(publisher.clone()), &pub_data);

        // Mark KYC as verified
        if let Some(mut kyc) = env
            .storage()
            .persistent()
            .get::<DataKey, KycRecord>(&DataKey::KycRecord(publisher.clone()))
        {
            kyc.verified = true;
            kyc.verified_at = Some(env.ledger().timestamp());
            env.storage()
                .persistent()
                .set(&DataKey::KycRecord(publisher.clone()), &kyc);
        }

        env.events().publish(
            (symbol_short!("publisher"), symbol_short!("verified")),
            publisher,
        );
    }

    /// Suspend a publisher (admin only)
    pub fn suspend_publisher(env: Env, admin: Address, publisher: Address) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            panic!("unauthorized");
        }

        let mut pub_data: Publisher = env
            .storage()
            .persistent()
            .get(&DataKey::Publisher(publisher.clone()))
            .expect("publisher not found");

        pub_data.status = VerificationStatus::Suspended;

        env.storage()
            .persistent()
            .set(&DataKey::Publisher(publisher), &pub_data);
    }

    /// Update publisher reputation score (admin only)
    pub fn update_reputation(env: Env, admin: Address, publisher: Address, score: u32) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            panic!("unauthorized");
        }

        if score > 1000 {
            panic!("invalid score");
        }

        let mut pub_data: Publisher = env
            .storage()
            .persistent()
            .get(&DataKey::Publisher(publisher.clone()))
            .expect("publisher not found");

        pub_data.reputation_score = score;
        pub_data.tier = Self::_score_to_tier(score);

        env.storage()
            .persistent()
            .set(&DataKey::Publisher(publisher), &pub_data);
    }

    /// Record impression (called by campaign orchestrator)
    pub fn record_impression(env: Env, caller: Address, publisher: Address, earning: i128) {
        // In production, restrict to campaign orchestrator contract only
        let mut pub_data: Publisher = env
            .storage()
            .persistent()
            .get(&DataKey::Publisher(publisher.clone()))
            .expect("publisher not found");

        match pub_data.status {
            VerificationStatus::Verified => {}
            _ => panic!("publisher not verified"),
        }

        pub_data.total_earnings += earning;
        pub_data.total_impressions += 1;
        pub_data.last_active = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&DataKey::Publisher(publisher), &pub_data);
    }

    // ============================================================
    // Read-Only Functions
    // ============================================================

    pub fn get_publisher(env: Env, publisher: Address) -> Option<Publisher> {
        env.storage()
            .persistent()
            .get(&DataKey::Publisher(publisher))
    }

    pub fn get_kyc(env: Env, publisher: Address) -> Option<KycRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::KycRecord(publisher))
    }

    pub fn is_verified(env: Env, publisher: Address) -> bool {
        if let Some(pub_data) = env
            .storage()
            .persistent()
            .get::<DataKey, Publisher>(&DataKey::Publisher(publisher))
        {
            matches!(pub_data.status, VerificationStatus::Verified)
        } else {
            false
        }
    }

    pub fn get_domain_owner(env: Env, domain: String) -> Option<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::DomainOwner(domain))
    }

    pub fn get_publisher_count(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::PublisherCount)
            .unwrap_or(0)
    }

    // ============================================================
    // Internal Helpers
    // ============================================================

    fn _score_to_tier(score: u32) -> PublisherTier {
        if score >= 800 {
            PublisherTier::Platinum
        } else if score >= 500 {
            PublisherTier::Gold
        } else if score >= 200 {
            PublisherTier::Silver
        } else {
            PublisherTier::Bronze
        }
    }
}

mod test;
