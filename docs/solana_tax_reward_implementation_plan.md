
# Solana Tax & Reward Program — Implementation Plan

## A. Repository Structure

```
solana-tax-reward/
├── programs/
│   └── tax_reward/             # Anchor-based Solana program
│       ├── src/
│       │   ├── entrypoint.rs
│       │   ├── lib.rs
│       │   ├── processor.rs
│       │   ├── state.rs
│       │   ├── error.rs
│       │   ├── instructions.rs
│       │   ├── swap.rs
│       │   └── utils.rs
│       ├── tests/              # Anchor Mocha + Rust tests
│       ├── Anchor.toml
│       └── Cargo.toml
├── clients/
│   ├── dapp/                   # Next.js + React dApp (formerly web)
│   └── batch-service/          # Node.js batch operations service (formerly relayer)
├── scripts/
│   ├── deploy/                 # Deployment scripts
│   └── maintenance/            # Maintenance & housekeeping scripts
├── .github/
│   ├── workflows/              # CI: build, test, audit, deploy pipelines
│   └── ISSUE_TEMPLATE.md
├── docs/
│   ├── ARCHITECTURE.md
│   ├── SECURITY.md
│   └── OPERATIONAL_RUNBOOK.md
├── CHANGELOG.md                # Release history
├── CONTRIBUTING.md             # Contribution guidelines
└── README.md                   # Project overview and setup
```

## B. Account & PDA Layout

| PDA Name             | Seeds                                               | Purpose                                                      |
|----------------------|-----------------------------------------------------|--------------------------------------------------------------|
| **Config**           | `["config", program_id, mint]`                      | Holds tax rates, owner, DEX config, paused flag.            |
| **GlobalState**      | `["global", program_id, mint]`                      | `total_supply: u64`, `cum_reward_per_token: u128`            |
| **TokenVault**       | `["token_vault", program_id, mint]`                 | Accrues taxed SPL tokens awaiting swap.                      |
| **RewardVault**      | `["reward_vault", program_id, mint]`                | Holds SOL for distribution.                                  |
| **UserInfo**         | `["user", program_id, user_pubkey, mint]`           | `last_cum: u128`, `balance_snapshot: u64` for reward pulls.  |

_All PDAs are rent-exempt on creation; front-end boots `UserInfo` when user first interacts._
Implement a `close_user_info` instruction to allow reclaiming rent and cleaning up stale accounts.

## C. Instruction Set (IDL)

```rust
/// Initialize the program; called once by deployer
initialize(
    ctx: Context<Initialize>,
    tax_rate_bps: u16,
    dex_program: Pubkey,
) -> ProgramResult

/// Handles buys & sells via DEX, taxes, swaps & updates rewards
taxed_swap_and_distribute(
    ctx: Context<TaxedSwap>,
    amount_in: u64,
    min_amount_out: u64,
) -> ProgramResult

/// Allows any holder to settle pending SOL rewards
claim_rewards(ctx: Context<Claim>) -> ProgramResult

/// Governance admin: update tax rates, pause/unpause
update_config(
    ctx: Context<UpdateConfig>,
    new_tax_rate_bps: u16,
    paused: bool,
) -> ProgramResult
```

## D. Cumulative Reward Accounting Pattern

1. **Global State** holds `cum_reward_per_token` scaled by 1e18.  
2. **On each swap**:  
   ```text
   delta_sol = swapped_amount;
   delta_cum = (delta_sol * 1e18) / total_supply;
   global.cum_reward_per_token += delta_cum;
   reward_vault.sol_balance += delta_sol;
   ```
3. **On each user interaction** (swap or claim):  
   ```text
   owed = user.balance_snapshot
        * (global.cum_reward_per_token - user.last_cum)
        / 1e18;
   transfer(RewardVault -> user_wallet, owed);
   user.last_cum = global.cum_reward_per_token;
   ```
   — This “lazy pull” avoids iterating all holders.

## E. Swap Integration

- **DexAdapter** (in `swap.rs`): unified interface to Jupiter & Serum.  
- **Slippage Protection**: caller submits `min_amount_out`; revert on unfavorable price.  
- **Fallback**: try alternative DEX routes if primary fails.  

## F. Governance & Upgradability

- **Config Account** controlled by a multisig (e.g. Gnosis Safe via Realms).  
- **Pause Flag**: emergency halt of all taxed transfers and swaps.  
- **Upgradeable BPF Loader**: upgrade authority set to multisig.

## G. Testing & CI/CD

1. **Local**: Anchor’s local validator + Mocha tests (`npm test`).  
2. **Mainnet-Fork**: run against a forked cluster to simulate real-world DEX behavior.  
3. **Property Tests**: use `proptest` to fuzz key invariants (e.g. no underflow).  
4. **CI Pipeline** (.github/workflows):  
   - `cargo fmt && cargo clippy && cargo test`  
   - `anchor test --skip-deploy`  
   - `npm run lint && npm run test`  
   - Security scan: `mantaray`, `oxygen audit`.

## H. Deployment & Monitoring

- **Deployment**: automated via `scripts/deploy.sh`, gated by CI.  
- **Monitoring**:  
  - On-chain metrics via Prometheus exporter (e.g. Solana-Prom).  
  - Alert on failed transactions or unusually high swap slippage.  
- **Operational Runbook**: documented in `docs/OPERATIONAL_RUNBOOK.md`.

## I. Security & Best Practices

- Use Anchor’s account constraint macros to validate inputs.  
- Protect against CPI reentrancy by using Solana’s built-in rent and compute checks.  
- Audit all CPIs, especially token and system transfers.  
- Cap per-transaction compute by splitting large operations into smaller relayer jobs if needed.
