//! PulsarTrack - Payment Processor (Soroban)
//! Multi-token payment support with fee distribution on Stellar.


#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    token, Address, Env, String,
};

// ============================================================
// Data Types
// ============================================================

#[contracttype]
#[derive(Clone)]
pub enum PaymentStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[contracttype]
#[derive(Clone)]
pub struct Payment {
    pub payer: Address,
    pub recipient: Address,
    pub token: Address,
    pub amount: i128,
    pub fee_charged: i128,
    pub status: PaymentStatus,
    pub created_at: u64,
    pub processed_at: Option<u64>,
}

#[contracttype]
#[derive(Clone)]
pub struct TokenConfig {
    pub enabled: bool,
    pub min_amount: i128,
    pub daily_limit: i128,
    pub daily_volume: i128,
}

#[contracttype]
#[derive(Clone)]
pub struct UserPaymentStats {
    pub total_payments: u64,
    pub total_spent: i128,
    pub last_payment: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct RevenueStats {
    pub total_fees_collected: i128,
    pub total_volume: i128,
    pub payment_count: u64,
}

// ============================================================
// Storage Keys
// ============================================================

#[contracttype]
pub enum DataKey {
    Admin,
    TreasuryAddress,
    NextPaymentId,
    PlatformFeeBps, // basis points (250 = 2.5%)
    TokenConfig(Address),
    Payment(u64),
    UserStats(Address),
    RevenueStats(Address),
    DailyVolume(Address, u64),
}

// ============================================================
// Contract
// ============================================================

#[contract]
pub struct PaymentProcessorContract;

#[contractimpl]
impl PaymentProcessorContract {
    /// Initialize the contract
    pub fn initialize(env: Env, admin: Address, treasury: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::TreasuryAddress, &treasury);
        env.storage()
            .instance()
            .set(&DataKey::NextPaymentId, &1u64);
        env.storage()
            .instance()
            .set(&DataKey::PlatformFeeBps, &250u32); // 2.5%
    }

    /// Whitelist a token for payments
    pub fn add_token(
        env: Env,
        admin: Address,
        token: Address,
        min_amount: i128,
        daily_limit: i128,
    ) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            panic!("unauthorized");
        }

        let config = TokenConfig {
            enabled: true,
            min_amount,
            daily_limit,
            daily_volume: 0,
        };

        env.storage()
            .persistent()
            .set(&DataKey::TokenConfig(token), &config);
    }

    /// Disable a payment token
    pub fn remove_token(env: Env, admin: Address, token: Address) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            panic!("unauthorized");
        }

        env.storage()
            .persistent()
            .remove(&DataKey::TokenConfig(token));
    }

    /// Process a payment
    pub fn process_payment(
        env: Env,
        payer: Address,
        recipient: Address,
        token: Address,
        amount: i128,
    ) -> u64 {
        payer.require_auth();

        if payer == recipient {
            panic!("cannot pay yourself");
        }

        if amount <= 0 {
            panic!("invalid amount");
        }

        let config: TokenConfig = env
            .storage()
            .persistent()
            .get(&DataKey::TokenConfig(token.clone()))
            .expect("token not whitelisted");

        if !config.enabled {
            panic!("token disabled");
        }

        if amount < config.min_amount {
            panic!("amount below minimum");
        }

        // Check daily limit
        let current_day = env.ledger().timestamp() / 86_400;
        let daily_key = DataKey::DailyVolume(token.clone(), current_day);
        let daily_vol: i128 = env.storage().temporary().get(&daily_key).unwrap_or(0);

        if daily_vol + amount > config.daily_limit {
            panic!("daily limit exceeded");
        }

        // Calculate fee
        let fee_bps: u32 = env
            .storage()
            .instance()
            .get(&DataKey::PlatformFeeBps)
            .unwrap_or(250);
        let fee = (amount * fee_bps as i128) / 10_000;
        let net_amount = amount - fee;

        let payment_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextPaymentId)
            .unwrap_or(1);

        // Execute token transfers
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&payer, &recipient, &net_amount);

        // Transfer fee to treasury
        if fee > 0 {
            let treasury: Address = env
                .storage()
                .instance()
                .get(&DataKey::TreasuryAddress)
                .unwrap();
            token_client.transfer(&payer, &treasury, &fee);
        }

        // Record payment
        let payment = Payment {
            payer: payer.clone(),
            recipient: recipient.clone(),
            token: token.clone(),
            amount,
            fee_charged: fee,
            status: PaymentStatus::Completed,
            created_at: env.ledger().timestamp(),
            processed_at: Some(env.ledger().timestamp()),
        };

        env.storage()
            .persistent()
            .set(&DataKey::Payment(payment_id), &payment);
        env.storage()
            .instance()
            .set(&DataKey::NextPaymentId, &(payment_id + 1));

        // Update daily volume
        env.storage()
            .temporary()
            .set(&daily_key, &(daily_vol + amount));

        // Update user stats
        Self::_update_user_stats(&env, &payer, amount);

        // Update revenue stats
        Self::_update_revenue_stats(&env, &token, fee, amount);

        env.events().publish(
            (symbol_short!("payment"), symbol_short!("processed")),
            (payment_id, payer, recipient, amount),
        );

        payment_id
    }

    /// Update platform fee (admin only)
    pub fn set_platform_fee(env: Env, admin: Address, fee_bps: u32) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            panic!("unauthorized");
        }
        if fee_bps > 1000 {
            panic!("fee too high"); // max 10%
        }
        env.storage()
            .instance()
            .set(&DataKey::PlatformFeeBps, &fee_bps);
    }

    // ============================================================
    // Read-Only Functions
    // ============================================================

    pub fn get_payment(env: Env, payment_id: u64) -> Option<Payment> {
        env.storage()
            .persistent()
            .get(&DataKey::Payment(payment_id))
    }

    pub fn get_user_stats(env: Env, user: Address) -> Option<UserPaymentStats> {
        env.storage()
            .persistent()
            .get(&DataKey::UserStats(user))
    }

    pub fn get_revenue_stats(env: Env, token: Address) -> Option<RevenueStats> {
        env.storage()
            .persistent()
            .get(&DataKey::RevenueStats(token))
    }

    pub fn get_token_config(env: Env, token: Address) -> Option<TokenConfig> {
        env.storage()
            .persistent()
            .get(&DataKey::TokenConfig(token))
    }

    // ============================================================
    // Internal Helpers
    // ============================================================

    fn _update_user_stats(env: &Env, user: &Address, amount: i128) {
        let key = DataKey::UserStats(user.clone());
        let mut stats: UserPaymentStats = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(UserPaymentStats {
                total_payments: 0,
                total_spent: 0,
                last_payment: 0,
            });

        stats.total_payments += 1;
        stats.total_spent += amount;
        stats.last_payment = env.ledger().timestamp();
        env.storage().persistent().set(&key, &stats);
    }

    fn _update_revenue_stats(env: &Env, token: &Address, fee: i128, volume: i128) {
        let key = DataKey::RevenueStats(token.clone());
        let mut stats: RevenueStats = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(RevenueStats {
                total_fees_collected: 0,
                total_volume: 0,
                payment_count: 0,
            });

        stats.total_fees_collected += fee;
        stats.total_volume += volume;
        stats.payment_count += 1;
        env.storage().persistent().set(&key, &stats);
    }
}

mod test;
