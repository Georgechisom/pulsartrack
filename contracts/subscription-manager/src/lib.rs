//! PulsarTrack - Subscription Manager (Soroban)
//! Manages platform subscription plans and billing on Stellar.

#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    token, Address, Env, String,
};

#[contracttype]
#[derive(Clone, PartialEq)]
pub enum SubscriptionTier {
    Starter,
    Growth,
    Business,
    Enterprise,
}

#[contracttype]
#[derive(Clone)]
pub struct SubscriptionPlan {
    pub tier: SubscriptionTier,
    pub name: String,
    pub price_monthly: i128,
    pub price_annual: i128,
    pub max_campaigns: u32,
    pub max_impressions_per_month: u64,
    pub max_publishers: u32,
    pub analytics_enabled: bool,
    pub api_access: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct Subscription {
    pub subscriber: Address,
    pub tier: SubscriptionTier,
    pub is_annual: bool,
    pub amount_paid: i128,
    pub started_at: u64,
    pub expires_at: u64,
    pub auto_renew: bool,
    pub campaigns_used: u32,
    pub impressions_used: u64,
}

#[contracttype]
pub enum DataKey {
    Admin,
    TokenAddress,
    TreasuryAddress,
    Plan(SubscriptionTier),
    Subscription(Address),
}

#[contract]
pub struct SubscriptionManagerContract;

#[contractimpl]
impl SubscriptionManagerContract {
    pub fn initialize(env: Env, admin: Address, token: Address, treasury: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::TokenAddress, &token);
        env.storage().instance().set(&DataKey::TreasuryAddress, &treasury);

        // Initialize default plans
        Self::_init_plans(&env);
    }

    pub fn subscribe(
        env: Env,
        subscriber: Address,
        tier: SubscriptionTier,
        is_annual: bool,
        auto_renew: bool,
    ) {
        subscriber.require_auth();

        let plan: SubscriptionPlan = env
            .storage()
            .persistent()
            .get(&DataKey::Plan(tier.clone()))
            .expect("plan not found");

        let amount = if is_annual {
            plan.price_annual
        } else {
            plan.price_monthly
        };

        let duration_secs = if is_annual {
            365 * 24 * 3600u64
        } else {
            30 * 24 * 3600u64
        };

        let token_addr: Address = env.storage().instance().get(&DataKey::TokenAddress).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        let treasury: Address = env.storage().instance().get(&DataKey::TreasuryAddress).unwrap();
        token_client.transfer(&subscriber, &treasury, &amount);

        let now = env.ledger().timestamp();
        let sub = Subscription {
            subscriber: subscriber.clone(),
            tier,
            is_annual,
            amount_paid: amount,
            started_at: now,
            expires_at: now + duration_secs,
            auto_renew,
            campaigns_used: 0,
            impressions_used: 0,
        };

        env.storage().persistent().set(&DataKey::Subscription(subscriber.clone()), &sub);

        env.events().publish(
            (symbol_short!("sub"), symbol_short!("subbed")),
            (subscriber, amount),
        );
    }

    pub fn cancel_subscription(env: Env, subscriber: Address) {
        subscriber.require_auth();

        let mut sub: Subscription = env
            .storage()
            .persistent()
            .get(&DataKey::Subscription(subscriber.clone()))
            .expect("subscription not found");

        sub.auto_renew = false;
        env.storage().persistent().set(&DataKey::Subscription(subscriber), &sub);
    }

    pub fn is_active(env: Env, subscriber: Address) -> bool {
        if let Some(sub) = env
            .storage()
            .persistent()
            .get::<DataKey, Subscription>(&DataKey::Subscription(subscriber))
        {
            sub.expires_at > env.ledger().timestamp()
        } else {
            false
        }
    }

    pub fn get_subscription(env: Env, subscriber: Address) -> Option<Subscription> {
        env.storage().persistent().get(&DataKey::Subscription(subscriber))
    }

    pub fn get_plan(env: Env, tier: SubscriptionTier) -> Option<SubscriptionPlan> {
        env.storage().persistent().get(&DataKey::Plan(tier))
    }

    fn _init_plans(env: &Env) {
        let plans = [
            (SubscriptionTier::Starter, "Starter", 99_000_000i128, 990_000_000i128, 5u32, 100_000u64, 10u32, false, false),
            (SubscriptionTier::Growth, "Growth", 299_000_000i128, 2_990_000_000i128, 25u32, 500_000u64, 50u32, true, false),
            (SubscriptionTier::Business, "Business", 799_000_000i128, 7_990_000_000i128, 100u32, 2_000_000u64, 200u32, true, true),
            (SubscriptionTier::Enterprise, "Enterprise", 1_999_000_000i128, 19_990_000_000i128, 1000u32, 10_000_000u64, 1000u32, true, true),
        ];

        for (tier, name, monthly, annual, max_campaigns, max_impressions, max_pubs, analytics, api) in plans {
            let plan = SubscriptionPlan {
                tier: tier.clone(),
                name: String::from_str(env, name),
                price_monthly: monthly,
                price_annual: annual,
                max_campaigns,
                max_impressions_per_month: max_impressions,
                max_publishers: max_pubs,
                analytics_enabled: analytics,
                api_access: api,
            };
            env.storage().persistent().set(&DataKey::Plan(tier), &plan);
        }
    }
}

mod test;
