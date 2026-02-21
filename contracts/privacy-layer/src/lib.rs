//! PulsarTrack - Privacy Layer (Soroban)
//! Zero-knowledge proofs and privacy-preserving ad targeting on Stellar.

#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Address, Bytes, BytesN, Env, String,
};

#[contracttype]
#[derive(Clone)]
pub struct PrivacyConsent {
    pub user: Address,
    pub data_processing: bool,
    pub targeted_ads: bool,
    pub analytics: bool,
    pub third_party_sharing: bool,
    pub consent_hash: BytesN<32>,
    pub consented_at: u64,
    pub expires_at: Option<u64>,
}

#[contracttype]
#[derive(Clone)]
pub struct AnonymousSegmentProof {
    pub proof_id: BytesN<32>,
    pub segment_ids: String,  // comma-separated segment IDs
    pub prover: Address,
    pub zkp_hash: BytesN<32>,  // zero-knowledge proof hash
    pub verified: bool,
    pub created_at: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct DataRequest {
    pub request_id: u64,
    pub requester: Address,
    pub data_type: String,
    pub purpose: String,
    pub approved: bool,
    pub requested_at: u64,
}

#[contracttype]
pub enum DataKey {
    Admin,
    RequestCounter,
    Consent(Address),
    Proof(BytesN<32>),
    DataRequest(u64),
}

#[contract]
pub struct PrivacyLayerContract;

#[contractimpl]
impl PrivacyLayerContract {
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::RequestCounter, &0u64);
    }

    pub fn set_consent(
        env: Env,
        user: Address,
        data_processing: bool,
        targeted_ads: bool,
        analytics: bool,
        third_party_sharing: bool,
        expires_in: Option<u64>,
    ) {
        user.require_auth();

        let consent_data = Bytes::from_slice(&env, b"consent");
        let consent_hash = env.crypto().sha256(&consent_data);

        let consent = PrivacyConsent {
            user: user.clone(),
            data_processing,
            targeted_ads,
            analytics,
            third_party_sharing,
            consent_hash: consent_hash.into(),
            consented_at: env.ledger().timestamp(),
            expires_at: expires_in.map(|d| env.ledger().timestamp() + d),
        };

        env.storage().persistent().set(&DataKey::Consent(user.clone()), &consent);

        env.events().publish(
            (symbol_short!("privacy"), symbol_short!("consent")),
            user,
        );
    }

    pub fn revoke_consent(env: Env, user: Address) {
        user.require_auth();
        env.storage().persistent().remove(&DataKey::Consent(user.clone()));

        env.events().publish(
            (symbol_short!("privacy"), symbol_short!("revoked")),
            user,
        );
    }

    pub fn submit_zkp(
        env: Env,
        prover: Address,
        segment_ids: String,
        zkp_hash: BytesN<32>,
    ) -> BytesN<32> {
        prover.require_auth();

        let proof_data = Bytes::from_slice(&env, b"proof");
        let proof_id = env.crypto().sha256(&proof_data);

        let proof = AnonymousSegmentProof {
            proof_id: proof_id.clone().into(),
            segment_ids,
            prover: prover.clone(),
            zkp_hash,
            verified: false,
            created_at: env.ledger().timestamp(),
        };

        env.storage().persistent().set(&DataKey::Proof(proof_id.clone().into()), &proof);

        env.events().publish(
            (symbol_short!("zkp"), symbol_short!("submitted")),
            prover,
        );

        proof_id.into()
    }

    pub fn verify_zkp(env: Env, admin: Address, proof_id: BytesN<32>) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            panic!("unauthorized");
        }

        let mut proof: AnonymousSegmentProof = env
            .storage()
            .persistent()
            .get(&DataKey::Proof(proof_id.clone()))
            .expect("proof not found");

        proof.verified = true;
        env.storage().persistent().set(&DataKey::Proof(proof_id), &proof);
    }

    pub fn has_consent(env: Env, user: Address, consent_type: String) -> bool {
        if let Some(consent) = env.storage().persistent().get::<DataKey, PrivacyConsent>(&DataKey::Consent(user)) {
            if let Some(expires) = consent.expires_at {
                if expires <= env.ledger().timestamp() {
                    return false;
                }
            }

            // Check specific consent type
            if consent_type == String::from_str(&env, "targeted_ads") {
                consent.targeted_ads
            } else if consent_type == String::from_str(&env, "analytics") {
                consent.analytics
            } else if consent_type == String::from_str(&env, "data_processing") {
                consent.data_processing
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn get_consent(env: Env, user: Address) -> Option<PrivacyConsent> {
        env.storage().persistent().get(&DataKey::Consent(user))
    }

    pub fn get_proof(env: Env, proof_id: BytesN<32>) -> Option<AnonymousSegmentProof> {
        env.storage().persistent().get(&DataKey::Proof(proof_id))
    }
}

mod test;
