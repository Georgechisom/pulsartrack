# Cross-Contract Validation Implementation

## Overview

This implementation adds cross-contract validation to the Campaign Orchestrator contract to ensure campaigns are properly validated across all related contracts before processing ad requests and bids.

## Problem Solved

Previously, the campaign orchestrator accepted campaign_id and processed ad requests without:

- Calling campaign-lifecycle to confirm the campaign is in Active status
- Calling escrow-vault to confirm sufficient budget remains
- Calling targeting-engine to confirm the publisher matches campaign targeting rules

This could lead to:

- Bids and ad requests processed for expired or paused campaigns
- Campaigns with zero remaining budget still serving ads
- Publishers explicitly excluded from a campaign's targeting still receiving ads

## Solution

### 1. Contract Address Storage

Added storage keys for external contract addresses:

- `LifecycleContract` - Campaign lifecycle management contract
- `EscrowContract` - Escrow vault for budget management
- `TargetingContract` - Targeting engine for publisher validation
- `AuctionContract` - Auction engine (for future use)

### 2. Admin Functions to Set Contract Addresses

Added four new admin-only functions to configure external contract addresses:

- `set_lifecycle_contract(admin, contract_address)`
- `set_escrow_contract(admin, contract_address)`
- `set_targeting_contract(admin, contract_address)`
- `set_auction_contract(admin, contract_address)`

These must be called after deployment to enable cross-contract validation.

### 3. Cross-Contract Validation Function

Implemented `_validate_campaign_cross_contract()` that performs three key validations:

#### a) Campaign Lifecycle Validation

```rust
// Calls campaign-lifecycle contract
- Verifies campaign exists in lifecycle contract
- Checks campaign is in Active state (not Draft, Paused, Cancelled, etc.)
- Validates campaign hasn't expired based on current_end_ledger
```

#### b) Escrow Budget Validation

```rust
// Calls escrow-vault contract
- Verifies escrow exists for the campaign
- Checks escrow has sufficient locked_amount > 0
- Validates escrow can be released (not disputed, time-lock passed, etc.)
- Ensures escrow is not in Disputed state (fraud detection)
```

#### c) Publisher Targeting Validation

```rust
// Calls targeting-engine contract
- Checks if targeting configuration exists for campaign
- Validates publisher has a targeting score
- Ensures publisher meets minimum reputation requirements
- Confirms publisher is not in excluded segments
```

### 4. Integration in record_view()

The validation is called at the beginning of `record_view()` before any state changes:

```rust
pub fn record_view(env: Env, campaign_id: u64, publisher: Address) {
    publisher.require_auth();

    // CROSS-CONTRACT VALIDATION
    Self::_validate_campaign_cross_contract(&env, campaign_id, &publisher);

    // ... rest of the function
}
```

This ensures that:

1. Invalid campaigns are rejected before any payment processing
2. Publishers not meeting targeting criteria are blocked
3. Campaigns with insufficient escrow budget cannot serve ads
4. Expired or paused campaigns are immediately rejected

## Technical Implementation

### Cross-Contract Calls

The implementation uses Soroban's `env.invoke_contract()` method to make cross-contract calls:

```rust
let result: Option<Val> = env.invoke_contract(
    &contract_address,
    &Symbol::new(env, "function_name"),
    SdkVec::from_array(env, [arg1.into_val(env), arg2.into_val(env)]),
);
```

### Error Handling

The validation function uses `panic!()` to halt execution if validation fails:

- `"campaign not found in lifecycle contract"` - Campaign doesn't exist
- `"escrow cannot be released - insufficient budget or conditions not met"` - Budget issues
- Warning event published if publisher has no targeting score

### Performance Considerations

Cross-contract calls are expensive in terms of gas/fees. The implementation:

- Only calls contracts if their addresses are configured (optional validation)
- Uses efficient single-call validation methods
- Fails fast on first validation error
- Caches contract addresses in instance storage

## Deployment & Configuration

### Step 1: Deploy Contracts

```bash
# Deploy all contracts
stellar contract deploy --wasm campaign-lifecycle.wasm
stellar contract deploy --wasm escrow-vault.wasm
stellar contract deploy --wasm targeting-engine.wasm
stellar contract deploy --wasm campaign-orchestrator.wasm
```

### Step 2: Initialize Orchestrator

```bash
stellar contract invoke \
  --id <ORCHESTRATOR_ID> \
  -- initialize \
  --admin <ADMIN_ADDRESS> \
  --token_address <TOKEN_ADDRESS>
```

### Step 3: Configure Contract Addresses

```bash
# Set lifecycle contract
stellar contract invoke \
  --id <ORCHESTRATOR_ID> \
  -- set_lifecycle_contract \
  --admin <ADMIN_ADDRESS> \
  --contract_address <LIFECYCLE_CONTRACT_ID>

# Set escrow contract
stellar contract invoke \
  --id <ORCHESTRATOR_ID> \
  -- set_escrow_contract \
  --admin <ADMIN_ADDRESS> \
  --contract_address <ESCROW_CONTRACT_ID>

# Set targeting contract
stellar contract invoke \
  --id <ORCHESTRATOR_ID> \
  -- set_targeting_contract \
  --admin <ADMIN_ADDRESS> \
  --contract_address <TARGETING_CONTRACT_ID>
```

## Testing

### Unit Tests

Test cases should cover:

1. ✅ Validation passes for active campaign with budget and valid publisher
2. ✅ Validation fails for non-existent campaign
3. ✅ Validation fails for paused/cancelled campaign
4. ✅ Validation fails for campaign with zero escrow budget
5. ✅ Validation fails for publisher not meeting targeting criteria
6. ✅ Validation is skipped if contract addresses not configured

### Integration Tests

```rust
#[test]
fn test_record_view_validates_lifecycle() {
    // Setup: Create campaign in lifecycle as Paused
    // Attempt: record_view on orchestrator
    // Expected: Panic with "campaign not active"
}

#[test]
fn test_record_view_validates_escrow() {
    // Setup: Create campaign with zero escrow budget
    // Attempt: record_view on orchestrator
    // Expected: Panic with "insufficient budget"
}

#[test]
fn test_record_view_validates_targeting() {
    // Setup: Create campaign with targeting rules excluding publisher
    // Attempt: record_view with excluded publisher
    // Expected: Panic or warning event
}
```

## Security Considerations

### 1. Admin-Only Configuration

Only the admin can set contract addresses, preventing malicious contract injection.

### 2. Contract Address Validation

Contract addresses should be validated before setting to ensure they implement the expected interface.

### 3. Fail-Safe Behavior

If a contract address is not set, validation for that contract is skipped. This allows:

- Gradual rollout of validation features
- Backward compatibility with existing deployments
- Graceful degradation if a contract is unavailable

### 4. Reentrancy Protection

Cross-contract calls could potentially enable reentrancy attacks. The implementation:

- Performs all validations before state changes
- Uses `require_auth()` to verify caller identity
- Doesn't allow callbacks during validation

## Future Enhancements

- [ ] Add caching layer for frequently accessed contract data
- [ ] Implement batch validation for multiple campaigns
- [ ] Add circuit breaker pattern for failing external contracts
- [ ] Create admin dashboard for monitoring validation failures
- [ ] Add metrics/events for validation performance tracking
- [ ] Implement fallback validation logic if external contracts are down
- [ ] Add support for custom validation rules per campaign type

## Files Changed

- ✅ `contracts/campaign-orchestrator/src/lib.rs` - Added cross-contract validation

### Key Changes:

1. Added contract address storage keys (LifecycleContract, EscrowContract, TargetingContract, AuctionContract)
2. Added 4 admin functions to set contract addresses
3. Implemented `_validate_campaign_cross_contract()` helper function
4. Integrated validation into `record_view()` function
5. Added warning events for edge cases

## Migration Guide

For existing deployments:

1. Deploy updated orchestrator contract
2. Call `set_lifecycle_contract()` with lifecycle contract address
3. Call `set_escrow_contract()` with escrow contract address
4. Call `set_targeting_contract()` with targeting contract address
5. Test with a single campaign before full rollout
6. Monitor events for validation failures

## References

- [Soroban Cross-Contract Calls](https://developers.stellar.org/docs/build/guides/conventions/cross-contract)
- [Campaign Lifecycle Contract](contracts/campaign-lifecycle/src/lib.rs)
- [Escrow Vault Contract](contracts/escrow-vault/src/lib.rs)
- [Targeting Engine Contract](contracts/targeting-engine/src/lib.rs)
