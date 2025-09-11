# Solana Tax & Reward System

[![Build Status](https://github.com/your-username/solana-tax-reward/workflows/CI/badge.svg)](https://github.com/your-username/solana-tax-reward/actions)
[![Rust Version](https://img.shields.io/badge/rust-1.70+-blue.svg)](https://www.rust-lang.org)
[![Anchor Version](https://img.shields.io/badge/anchor-0.27.0-purple.svg)](https://anchor-lang.com/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A comprehensive Solana-based DeFi system that implements a **tax-and-reward mechanism** for SPL token trades. The system automatically taxes token swaps, converts collected tokens to SOL via DEX integration, and redistributes rewards to token holders proportionally.

## Key Features

- ** Automated Tax Collection**: Configurable tax rates (0-100%) on token swaps
- ** SOL Reward Distribution**: Collected tokens are swapped to SOL and distributed to holders
- ** Multi-DEX Integration**: Primary Jupiter support with Serum fallback
- ** Real-time Reward Accounting**: Cumulative reward per token algorithm (scaled by 1e18)
- ** Security First**: Comprehensive overflow protection and pause mechanisms
- ** Production Ready**: Full CI/CD, monitoring, and operational runbooks

## System Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Web dApp      │    │ Batch Service   │    │ Solana Program  │
│   (Next.js)     │◄──►│   (Node.js)     │◄──►│   (Anchor)      │
│                 │    │                 │    │                 │
│ • User Interface│    │ • Automation    │    │ • Tax Logic     │
│ • Wallet Connect│    │ • Monitoring    │    │ • Reward Dist.  │
│ • Transaction   │    │ • Batch Ops     │    │ • DEX Integration│
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                                        │
                                                        ▼
                                          ┌─────────────────┐
                                          │  DEX Protocols  │
                                          │                 │
                                          │ • Jupiter (1°)  │
                                          │ • Serum (2°)    │
                                          └─────────────────┘
```

##  Repository Structure

```
solana-tax-reward/
├──  programs/tax_reward/          # Anchor Solana program
│   ├── src/
│   │   ├── lib.rs                   # Main program logic
│   │   ├── state.rs                 # Account structures
│   │   ├── instructions.rs          # Context definitions
│   │   ├── swap.rs                  # DEX integration
│   │   └── error.rs                 # Custom errors
│   ├── tests/                       # Program tests
│   ├── Cargo.toml                   # Dependencies
│   └── Anchor.toml                  # Anchor config
│
├── clients/
│   ├── dapp/                        # Next.js 15 + React 19 frontend
│   │   ├── src/app/                 # App Router structure
│   │   ├── src/lib/                 # Utilities & hooks
│   │   ├── cypress/                 # E2E tests
│   │   └── package.json             # Dependencies
│   └── batch-service/               # Node.js automation service
│       ├── src/index.js             # Express server
│       ├── tests/                   # Service tests
│       └── package.json             # Dependencies
│
├── scripts/
│   ├── deploy/deploy.sh             # Deployment automation
│   ├── maintenance/monitoring.sh    # Health checks
│   └── benchmark/compute_bench.sh   # Performance testing
│
├── tests/                        # Integration test suites
│   ├── anchor_tests.rs              # Anchor program tests
│   ├── e2e_tests.rs                 # End-to-end scenarios
│   ├── property_tests.rs            # Property-based testing
│   └── integration_tests.rs         # Cross-component tests
│
├── docs/
│   ├── ARCHITECTURE.md              # System design
│   ├── OPERATIONAL_RUNBOOK.md       # Deployment guide
│   ├── SECURITY.md                  # Security practices
│   ├── TESTING_FRAMEWORK.md         # Testing strategy
│   └── monitoring/                  # Prometheus config
│
├──  .github/workflows/            # CI/CD pipelines
│   ├── ci.yml                       # Continuous integration
│   └── deploy.yml                   # Deployment workflow
│
└── Configuration files
    ├── Anchor.toml                  # Workspace config
    ├── Cargo.toml                   # Rust workspace
    ├── CHANGELOG.md                 # Version history
    └── CONTRIBUTING.md              # Contribution guide
```

## Quick Start

### Prerequisites

- **Rust** 1.70+ with `cargo`
- **Solana CLI** 1.14.17+
- **Anchor Framework** 0.27.0+
- **Node.js** 18+ with `npm`
- **Git** for version control

### Clone and Setup

```bash
git clone https://github.com/your-username/solana-tax-reward.git
cd solana-tax-reward

# Verify Rust installation
rustc --version
solana --version
anchor --version
```

### Build the Solana Program

```bash
cd programs/tax_reward

# Install dependencies and build
cargo check                          # Verify compilation
anchor build                         # Build the program
anchor test --skip-deploy           # Run unit tests
```

### Deploy to Localnet (Development)

```bash
# Start local validator
solana-test-validator

# Deploy program
anchor deploy --provider.cluster localnet

# Run integration tests
anchor test
```

### Start the Web dApp

```bash
cd clients/dapp

npm install                         # Install dependencies
npm run dev                         # Start development server
# Visit http://localhost:3000
```

### Start the Batch Service

```bash
cd clients/batch-service

npm install                         # Install dependencies
npm start                           # Start the service
# Service runs on http://localhost:3001
```

## Testing

### Program Tests
```bash
# Unit tests
cargo test --manifest-path programs/tax_reward/Cargo.toml

# Property-based tests
cargo test --manifest-path programs/tax_reward/Cargo.toml -- --ignored

# Anchor integration tests
anchor test --skip-deploy
```

### Client Tests
```bash
# dApp tests
cd clients/dapp
npm test                            # Unit tests
npm run e2e:test                    # Cypress E2E tests

# Batch service tests  
cd clients/batch-service
npm test                            # Jest tests
npm run e2e:test                    # Integration tests
```

### Full Test Suite
```bash
# Run everything (mimics CI pipeline)
cargo fmt --manifest-path programs/tax_reward/Cargo.toml -- --check
cargo clippy --manifest-path programs/tax_reward/Cargo.toml --all -- -D warnings
anchor test --skip-deploy
cargo test --manifest-path programs/tax_reward/Cargo.toml -- --ignored
```

## Development Commands

### Anchor Program
```bash
anchor build                        # Build program
anchor deploy                       # Deploy to configured cluster
anchor test                         # Run all tests
anchor clean                        # Clean build artifacts
```

### Code Quality
```bash
cargo fmt                           # Format Rust code
cargo clippy                        # Lint Rust code
npm run lint                        # Lint TypeScript/JavaScript
npm run format                      # Format client code
```

### Deployment
```bash
# Deploy to devnet
./scripts/deploy/deploy.sh devnet ~/.config/solana/id.json

# Deploy to mainnet (requires multisig)
./scripts/deploy/deploy.sh mainnet ~/.config/solana/mainnet-key.json
```

## Monitoring & Operations

- **Health Checks**: `./scripts/maintenance/monitoring.sh`
- **Metrics**: Prometheus metrics at `/api/metrics`
- **Performance**: `./scripts/benchmark/compute_bench.sh`
- **Logs**: Centralized logging via Winston/Pino

See [OPERATIONAL_RUNBOOK.md](./docs/OPERATIONAL_RUNBOOK.md) for detailed procedures.

## Security

- **Math Safety**: All arithmetic operations use checked math
- **Access Control**: PDA-based authority validation
- **Emergency Stop**: Program pause functionality
- **Audit Trail**: Comprehensive event logging

See [SECURITY.md](./docs/SECURITY.md) for security practices and reporting.

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](./CONTRIBUTING.md) for:

- Development setup
- Code style guidelines
- Testing requirements
- Pull request process

## 📚 Documentation

- [**Architecture Guide**](./docs/ARCHITECTURE.md) - System design and components
- [**Operational Runbook**](./docs/OPERATIONAL_RUNBOOK.md) - Deployment and maintenance
- [**Testing Framework**](./docs/TESTING_FRAMEWORK.md) - Testing strategy and tools
- [**API Documentation**](./docs/API.md) - Program instructions and client APIs

## Project Status

- **Core Program**: Functional and tested
- **DEX Integration**: Jupiter + Serum support
- **Web dApp**: Next.js 15 with full functionality
- **Batch Service**: Automated operations
- **CI/CD**: GitHub Actions pipeline
- **Monitoring**: Prometheus + Grafana ready
- **Mainnet Deployment**: In preparation

## Links

- [Solana Documentation](https://docs.solana.com/)
- [Anchor Framework](https://anchor-lang.com/)
- [Jupiter Protocol](https://jup.ag/)
- [Serum DEX](https://serum-academy.com/)

