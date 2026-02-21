//! PulsarTrack - Liquidity Pool (Soroban)
//! Ad budget liquidity pool for campaign funding on Stellar.

#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    token, Address, Env,
};

#[contracttype]
#[derive(Clone)]
pub struct PoolState {
    pub total_liquidity: i128,
    pub total_borrowed: i128,
    pub reserve_factor: u32,   // percentage kept as reserve
    pub utilization_rate: u32, // percentage borrowed
    pub borrow_rate_bps: u32,  // annual rate in basis points
    pub last_updated: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct ProviderPosition {
    pub provider: Address,
    pub deposited: i128,
    pub shares: i128,
    pub deposited_at: u64,
    pub last_claim: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct BorrowPosition {
    pub borrower: Address,
    pub campaign_id: u64,
    pub borrowed: i128,
    pub interest_accrued: i128,
    pub borrowed_at: u64,
    pub due_at: u64,
}

#[contracttype]
pub enum DataKey {
    Admin,
    TokenAddress,
    PoolState,
    TotalShares,
    Provider(Address),
    Borrow(u64),        // campaign_id
    BorrowCount,
}

#[contract]
pub struct LiquidityPoolContract;

#[contractimpl]
impl LiquidityPoolContract {
    pub fn initialize(env: Env, admin: Address, token: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::TokenAddress, &token);
        env.storage().instance().set(&DataKey::TotalShares, &0i128);
        env.storage().instance().set(&DataKey::PoolState, &PoolState {
            total_liquidity: 0,
            total_borrowed: 0,
            reserve_factor: 10,
            utilization_rate: 0,
            borrow_rate_bps: 500, // 5% annual
            last_updated: env.ledger().timestamp(),
        });
    }

    pub fn deposit(env: Env, provider: Address, amount: i128) -> i128 {
        provider.require_auth();

        if amount <= 0 {
            panic!("invalid amount");
        }

        let token_addr: Address = env.storage().instance().get(&DataKey::TokenAddress).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        token_client.transfer(&provider, &env.current_contract_address(), &amount);

        let mut pool: PoolState = env.storage().instance().get(&DataKey::PoolState).unwrap();
        let total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);

        // Calculate shares (1:1 for first deposit)
        let shares = if pool.total_liquidity == 0 || total_shares == 0 {
            amount
        } else {
            (amount * total_shares) / pool.total_liquidity
        };

        pool.total_liquidity += amount;
        pool.last_updated = env.ledger().timestamp();
        env.storage().instance().set(&DataKey::PoolState, &pool);
        env.storage().instance().set(&DataKey::TotalShares, &(total_shares + shares));

        let mut position: ProviderPosition = env
            .storage()
            .persistent()
            .get(&DataKey::Provider(provider.clone()))
            .unwrap_or(ProviderPosition {
                provider: provider.clone(),
                deposited: 0,
                shares: 0,
                deposited_at: env.ledger().timestamp(),
                last_claim: env.ledger().timestamp(),
            });

        position.deposited += amount;
        position.shares += shares;
        env.storage().persistent().set(&DataKey::Provider(provider.clone()), &position);

        env.events().publish(
            (symbol_short!("pool"), symbol_short!("deposit")),
            (provider, amount, shares),
        );

        shares
    }

    pub fn withdraw(env: Env, provider: Address, shares: i128) -> i128 {
        provider.require_auth();

        let mut position: ProviderPosition = env
            .storage()
            .persistent()
            .get(&DataKey::Provider(provider.clone()))
            .expect("no position");

        if position.shares < shares {
            panic!("insufficient shares");
        }

        let mut pool: PoolState = env.storage().instance().get(&DataKey::PoolState).unwrap();
        let total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);

        let amount = (shares * pool.total_liquidity) / total_shares;
        let available = pool.total_liquidity - pool.total_borrowed;

        if amount > available {
            panic!("insufficient liquidity");
        }

        pool.total_liquidity -= amount;
        pool.last_updated = env.ledger().timestamp();
        env.storage().instance().set(&DataKey::PoolState, &pool);
        env.storage().instance().set(&DataKey::TotalShares, &(total_shares - shares));

        position.shares -= shares;
        position.deposited = position.deposited.saturating_sub(amount);
        env.storage().persistent().set(&DataKey::Provider(provider.clone()), &position);

        let token_addr: Address = env.storage().instance().get(&DataKey::TokenAddress).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        token_client.transfer(&env.current_contract_address(), &provider, &amount);

        amount
    }

    pub fn borrow(env: Env, borrower: Address, campaign_id: u64, amount: i128, duration_secs: u64) {
        borrower.require_auth();

        let mut pool: PoolState = env.storage().instance().get(&DataKey::PoolState).unwrap();
        let available = pool.total_liquidity - pool.total_borrowed;

        if amount > available {
            panic!("insufficient liquidity");
        }

        if env.storage().persistent().has(&DataKey::Borrow(campaign_id)) {
            panic!("already has borrow");
        }

        pool.total_borrowed += amount;
        pool.utilization_rate = ((pool.total_borrowed * 100) / pool.total_liquidity) as u32;
        pool.last_updated = env.ledger().timestamp();
        env.storage().instance().set(&DataKey::PoolState, &pool);

        let now = env.ledger().timestamp();
        let borrow = BorrowPosition {
            borrower: borrower.clone(),
            campaign_id,
            borrowed: amount,
            interest_accrued: 0,
            borrowed_at: now,
            due_at: now + duration_secs,
        };

        env.storage().persistent().set(&DataKey::Borrow(campaign_id), &borrow);

        let token_addr: Address = env.storage().instance().get(&DataKey::TokenAddress).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        token_client.transfer(&env.current_contract_address(), &borrower, &amount);
    }

    pub fn repay(env: Env, borrower: Address, campaign_id: u64, amount: i128) {
        borrower.require_auth();

        let borrow: BorrowPosition = env
            .storage()
            .persistent()
            .get(&DataKey::Borrow(campaign_id))
            .expect("borrow not found");

        if borrow.borrower != borrower {
            panic!("unauthorized");
        }

        let token_addr: Address = env.storage().instance().get(&DataKey::TokenAddress).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        token_client.transfer(&borrower, &env.current_contract_address(), &amount);

        let mut pool: PoolState = env.storage().instance().get(&DataKey::PoolState).unwrap();
        pool.total_borrowed -= amount.min(borrow.borrowed);
        if pool.total_liquidity > 0 {
            pool.utilization_rate = ((pool.total_borrowed * 100) / pool.total_liquidity) as u32;
        }
        pool.total_liquidity += amount;
        pool.last_updated = env.ledger().timestamp();
        env.storage().instance().set(&DataKey::PoolState, &pool);

        env.storage().persistent().remove(&DataKey::Borrow(campaign_id));
    }

    pub fn get_pool_state(env: Env) -> PoolState {
        env.storage().instance().get(&DataKey::PoolState).expect("not initialized")
    }

    pub fn get_provider_position(env: Env, provider: Address) -> Option<ProviderPosition> {
        env.storage().persistent().get(&DataKey::Provider(provider))
    }

    pub fn get_borrow(env: Env, campaign_id: u64) -> Option<BorrowPosition> {
        env.storage().persistent().get(&DataKey::Borrow(campaign_id))
    }
}

mod test;
