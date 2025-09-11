# Solana Tax & Reward

This repository contains an on‐chain Solana program that taxes SPL token trades and redistributes collected fees as SOL rewards, along with a web dApp and batch service.  

## Repository Layout

├── programs/  
│   └── tax_reward/ – Anchor program (Rust)  
│       ├── src/ – program modules  
│       ├── tests/ – program tests  
│       ├── Anchor.toml  
│       └── Cargo.toml  

├── clients/  
│   ├── dapp/ – Next.js + React web interface  
│   └── batch-service/ – Node.js batch operations  

├── scripts/  
│   ├── deploy/ – deployment scripts  
│   └── maintenance/ – maintenance scripts  

├── .github/  
│   ├── workflows/ – CI/CD pipelines  
│   └── ISSUE_TEMPLATE.md  

├── docs/ – design & runbook documents  
├── CHANGELOG.md  
├── CONTRIBUTING.md  

## Getting Started

### On‐chain Program

```
cd programs/tax_reward
anchor build
anchor test
```

### Web dApp

```
cd clients/dapp
npm install
npm run dev
```

### Batch Service

```
cd clients/batch-service
npm install
npm start
```

## License

MIT