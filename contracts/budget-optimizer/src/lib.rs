//! PulsarTrack - Budget Optimizer (Soroban)
//! Automated campaign budget optimization and allocation on Stellar.

#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Address, Env,
};

#[contracttype]
#[derive(Clone)]
pub struct BudgetAllocation {
    pub campaign_id: u64,
    pub total_budget: i128,
    pub daily_budget: i128,
    pub hourly_budget: i128,
    pub spent_today: i128,
    pub spent_total: i128,
    pub optimization_mode: OptimizationMode,
    pub target_cpa: i128,    // Target cost per acquisition
    pub target_ctr: u32,     // Target CTR * 10000
    pub last_optimized: u64,
}

#[contracttype]
#[derive(Clone)]
pub enum OptimizationMode {
    ManualCpc,       // Manual cost per click
    AutoCpm,         // Automatic CPM
    TargetCpa,       // Target cost per acquisition
    MaxConversions,  // Maximize conversions
    MaxReach,        // Maximize reach
}

#[contracttype]
#[derive(Clone)]
pub struct OptimizationLog {
    pub campaign_id: u64,
    pub old_daily_budget: i128,
    pub new_daily_budget: i128,
    pub reason: String,
    pub optimized_at: u64,
}

use soroban_sdk::String;

#[contracttype]
pub enum DataKey {
    Admin,
    OracleAddress,
    Allocation(u64),
    OptLog(u64, u32),  // campaign_id, log_index
    OptLogCount(u64),
}

#[contract]
pub struct BudgetOptimizerContract;

#[contractimpl]
impl BudgetOptimizerContract {
    pub fn initialize(env: Env, admin: Address, oracle: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::OracleAddress, &oracle);
    }

    pub fn set_budget_allocation(
        env: Env,
        advertiser: Address,
        campaign_id: u64,
        total_budget: i128,
        daily_budget: i128,
        optimization_mode: OptimizationMode,
        target_cpa: i128,
        target_ctr: u32,
    ) {
        advertiser.require_auth();

        if daily_budget > total_budget {
            panic!("daily budget exceeds total");
        }

        let allocation = BudgetAllocation {
            campaign_id,
            total_budget,
            daily_budget,
            hourly_budget: daily_budget / 24,
            spent_today: 0,
            spent_total: 0,
            optimization_mode,
            target_cpa,
            target_ctr,
            last_optimized: env.ledger().timestamp(),
        };

        env.storage().persistent().set(&DataKey::Allocation(campaign_id), &allocation);
    }

    pub fn optimize_budget(
        env: Env,
        oracle: Address,
        campaign_id: u64,
        new_daily_budget: i128,
        reason: String,
    ) {
        oracle.require_auth();
        let stored_oracle: Address = env.storage().instance().get(&DataKey::OracleAddress).unwrap();
        if oracle != stored_oracle {
            panic!("unauthorized");
        }

        let mut allocation: BudgetAllocation = env
            .storage()
            .persistent()
            .get(&DataKey::Allocation(campaign_id))
            .expect("allocation not found");

        let old_daily = allocation.daily_budget;

        // Ensure new daily budget doesn't exceed total remaining
        let remaining = allocation.total_budget - allocation.spent_total;
        let capped_daily = new_daily_budget.min(remaining);

        allocation.daily_budget = capped_daily;
        allocation.hourly_budget = capped_daily / 24;
        allocation.last_optimized = env.ledger().timestamp();

        env.storage().persistent().set(&DataKey::Allocation(campaign_id), &allocation);

        // Log the optimization
        let count: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::OptLogCount(campaign_id))
            .unwrap_or(0);

        let log = OptimizationLog {
            campaign_id,
            old_daily_budget: old_daily,
            new_daily_budget: capped_daily,
            reason,
            optimized_at: env.ledger().timestamp(),
        };

        env.storage().persistent().set(&DataKey::OptLog(campaign_id, count), &log);
        env.storage().persistent().set(&DataKey::OptLogCount(campaign_id), &(count + 1));

        env.events().publish(
            (symbol_short!("budget"), symbol_short!("optimized")),
            (campaign_id, capped_daily),
        );
    }

    pub fn record_spend(env: Env, admin: Address, campaign_id: u64, amount: i128) {
        admin.require_auth();

        let mut allocation: BudgetAllocation = env
            .storage()
            .persistent()
            .get(&DataKey::Allocation(campaign_id))
            .expect("allocation not found");

        allocation.spent_today += amount;
        allocation.spent_total += amount;

        // Reset daily spend if new day
        let hour = env.ledger().timestamp() / 86_400;
        let _ = hour; // Could be used for daily tracking

        env.storage().persistent().set(&DataKey::Allocation(campaign_id), &allocation);
    }

    pub fn get_allocation(env: Env, campaign_id: u64) -> Option<BudgetAllocation> {
        env.storage().persistent().get(&DataKey::Allocation(campaign_id))
    }

    pub fn can_spend(env: Env, campaign_id: u64, amount: i128) -> bool {
        if let Some(alloc) = env.storage().persistent().get::<DataKey, BudgetAllocation>(&DataKey::Allocation(campaign_id)) {
            alloc.spent_today + amount <= alloc.daily_budget
                && alloc.spent_total + amount <= alloc.total_budget
        } else {
            false
        }
    }
}

mod test;
