//! PulsarTrack - Audience Segments (Soroban)
//! Privacy-preserving audience segmentation and targeting on Stellar.

#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Address, Env, String, Vec,
};

#[contracttype]
#[derive(Clone)]
pub struct Segment {
    pub segment_id: u64,
    pub name: String,
    pub description: String,
    pub criteria_hash: String,  // IPFS hash of targeting criteria
    pub creator: Address,
    pub member_count: u64,
    pub is_public: bool,
    pub created_at: u64,
    pub last_updated: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct SegmentMembership {
    pub segment_id: u64,
    pub member: Address,
    pub joined_at: u64,
    pub score: u32,  // relevance score 0-1000
}

#[contracttype]
pub enum DataKey {
    Admin,
    SegmentCounter,
    Segment(u64),
    Membership(u64, Address),  // segment_id, member
    MemberCount(u64),
}

#[contract]
pub struct AudienceSegmentsContract;

#[contractimpl]
impl AudienceSegmentsContract {
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::SegmentCounter, &0u64);
    }

    pub fn create_segment(
        env: Env,
        creator: Address,
        name: String,
        description: String,
        criteria_hash: String,
        is_public: bool,
    ) -> u64 {
        creator.require_auth();

        let counter: u64 = env.storage().instance().get(&DataKey::SegmentCounter).unwrap_or(0);
        let segment_id = counter + 1;

        let segment = Segment {
            segment_id,
            name,
            description,
            criteria_hash,
            creator: creator.clone(),
            member_count: 0,
            is_public,
            created_at: env.ledger().timestamp(),
            last_updated: env.ledger().timestamp(),
        };

        env.storage().persistent().set(&DataKey::Segment(segment_id), &segment);
        env.storage().instance().set(&DataKey::SegmentCounter, &segment_id);

        env.events().publish(
            (symbol_short!("segment"), symbol_short!("created")),
            (segment_id, creator),
        );

        segment_id
    }

    pub fn add_member(env: Env, admin: Address, segment_id: u64, member: Address, score: u32) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();

        let segment: Segment = env
            .storage()
            .persistent()
            .get(&DataKey::Segment(segment_id))
            .expect("segment not found");

        // Either admin or segment creator can add members
        if admin != stored_admin && admin != segment.creator {
            panic!("unauthorized");
        }

        if env.storage().persistent().has(&DataKey::Membership(segment_id, member.clone())) {
            panic!("already a member");
        }

        let membership = SegmentMembership {
            segment_id,
            member: member.clone(),
            joined_at: env.ledger().timestamp(),
            score,
        };

        env.storage().persistent().set(&DataKey::Membership(segment_id, member), &membership);

        let count: u64 = env.storage().persistent().get(&DataKey::MemberCount(segment_id)).unwrap_or(0);
        env.storage().persistent().set(&DataKey::MemberCount(segment_id), &(count + 1));
    }

    pub fn remove_member(env: Env, admin: Address, segment_id: u64, member: Address) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();

        let segment: Segment = env
            .storage()
            .persistent()
            .get(&DataKey::Segment(segment_id))
            .expect("segment not found");

        if admin != stored_admin && admin != segment.creator {
            panic!("unauthorized");
        }

        env.storage().persistent().remove(&DataKey::Membership(segment_id, member));

        let count: u64 = env.storage().persistent().get(&DataKey::MemberCount(segment_id)).unwrap_or(0);
        if count > 0 {
            env.storage().persistent().set(&DataKey::MemberCount(segment_id), &(count - 1));
        }
    }

    pub fn is_member(env: Env, segment_id: u64, member: Address) -> bool {
        env.storage().persistent().has(&DataKey::Membership(segment_id, member))
    }

    pub fn get_segment(env: Env, segment_id: u64) -> Option<Segment> {
        env.storage().persistent().get(&DataKey::Segment(segment_id))
    }

    pub fn get_membership(env: Env, segment_id: u64, member: Address) -> Option<SegmentMembership> {
        env.storage().persistent().get(&DataKey::Membership(segment_id, member))
    }

    pub fn get_segment_count(env: Env) -> u64 {
        env.storage().instance().get(&DataKey::SegmentCounter).unwrap_or(0)
    }

    pub fn get_member_count(env: Env, segment_id: u64) -> u64 {
        env.storage().persistent().get(&DataKey::MemberCount(segment_id)).unwrap_or(0)
    }
}

mod test;
