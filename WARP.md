# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Common Commands

### Anchor Program Development
```bash
# Build the Anchor program
anchor build

# Run program tests (no deployment needed)
anchor test --skip-deploy

# Deploy to localnet
anchor deploy --provider.cluster localnet

# Deploy to specific cluster with custom wallet
./scripts/deploy/deploy.sh mainnet ~/.config/solana/id.json

# Run property-based tests
cargo test -- --ignored

# Format and lint Rust code
cargo fmt -- --check
cargo clippy --all -- -D warnings
```

### Client Development
```bash
# dApp (Next.js)
cd clients/dapp
npm install
npm run dev          # Start development server with Turbopack
npm run build        # Production build
npm run lint         # ESLint check
npm run e2e:test     # Cypress E2E tests

# Batch Service (Node.js)
cd clients/batch-service
npm install
npm start           # Start service
npm run lint        # ESLint check
npm run e2e:test    # Jest E2E tests
```

### Testing & CI
```bash
# Run all tests (mimics CI pipeline)
cargo fmt -- --check && cargo clippy --all -- -D warnings
anchor test --skip-deploy
cargo test -- --ignored

# Run client tests
cd clients/dapp && npm test
cd clients/batch-service && npm test
```

## Architecture Overview

### Program Structure (Anchor-based Solana Program)
The core is an Anchor program (`programs/tax_reward/`) that implements a tax-and-reward mechanism for SPL token trades:

**Key Components:**
- **State Management**: Three main account types managed via PDAs:
  - `Config`: Tax rates, owner authority, DEX configuration, pause flag
  - `GlobalState`: Total supply tracking and cumulative reward per token (scaled 1e18)
  - `UserInfo`: Per-user reward accounting with balance snapshots
  
- **Core Instructions**:
  - `initialize`: One-time setup of program state
  - `taxed_swap_and_distribute`: Main trading function that taxes trades, swaps tokens to SOL, distributes rewards
  - `claim_rewards`: Allows users to claim pending SOL rewards
  - `update_config`: Admin function for tax rate changes and pause/unpause
  - `close_user_info`: Cleanup stale user accounts

- **DEX Integration**: Dual-strategy swap mechanism in `swap.rs`:
  - Primary: Jupiter aggregator 
  - Fallback: Serum DEX
  - Handles token-to-SOL conversion for reward distribution

### Client Architecture
- **dApp**: Next.js 15 + React 19 frontend with TailwindCSS, TypeScript, state management via Zustand
- **Batch Service**: Node.js Express service for automated operations, monitoring via Prometheus

### PDA Seed Patterns
All program accounts use predictable PDA seeds:
- Config: `["config", program_id, mint_key]`
- Global State: `["global", program_id, mint_key]`  
- Token Vault: `["token_vault", program_id, mint_key]`
- Reward Vault: `["reward_vault", program_id, mint_key]`
- User Info: `["user", program_id, user_wallet, mint_key]`

### Reward Distribution Algorithm
Uses cumulative reward per token accounting:
1. On each swap, collected tax tokens are converted to SOL
2. SOL rewards update global `cum_reward_per_token` (scaled 1e18)
3. User rewards calculated as: `user_balance * (global_cum - user_last_cum) / SCALE`
4. Lazy reward claiming: rewards auto-distributed during swaps or explicit claims

## Important Development Notes

### Program Configuration
- Program ID placeholder: `"ReplaceWithProgramID"` in `Anchor.toml` (update after first deploy)
- Upgrade authority uses multisig: `"ReplaceWithMultisigPubkey"`
- Default cluster: `localnet` (change for mainnet deployment)

### Critical Dependencies
- Anchor framework 0.27.0
- Solana SDK 1.14.17
- Node.js 18+ for clients
- Rust with overflow-checks enabled in release profile

### Testing Strategy
- Unit tests: `cargo test`
- Integration tests: `anchor test --skip-deploy`
- Property tests: `cargo test -- --ignored` (uses proptest crate)
- E2E tests: Cypress (dApp) and Jest (batch-service)
- Mainnet-fork testing in CI pipeline

### Security Considerations
- Program can be paused via config flag (emergency stop)
- All math operations use checked arithmetic to prevent overflows
- PDA-based account derivation ensures proper authority checks
- CI includes security scanning with failure gates

### Monitoring & Operations
- Prometheus metrics integration in both clients
- Operational procedures documented in `docs/OPERATIONAL_RUNBOOK.md`
- CI/CD handles deployments, testing, and security checks
- Load testing via `wrk` tool included in CI

### Compute Budget Management
The `taxed_swap_and_distribute` instruction requests increased compute budget (400,000 units) due to complex operations involving DEX swaps and reward calculations.

## Key Files to Understand
- `programs/tax_reward/src/lib.rs`: Main program logic and instruction handlers
- `programs/tax_reward/src/state.rs`: Account structures and data layouts  
- `programs/tax_reward/src/swap.rs`: DEX integration and swap logic
- `programs/tax_reward/src/instructions.rs`: Anchor account validation contexts
- `docs/ARCHITECTURE.md`: High-level system design
- `docs/OPERATIONAL_RUNBOOK.md`: Deployment and maintenance procedures
