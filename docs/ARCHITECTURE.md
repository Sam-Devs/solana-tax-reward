# Architecture

This document describes the high‐level design:

- **Programs**: Anchor-based Solana program under `programs/tax_reward`.
- **Clients**:  
  - dApp (Next.js + React)  
  - Batch service (Node.js)  
- **Data Flow**: Tax → Swap → Reward accounting → Distribution  
- **Account Layout**: PDAs for config, vaults, global state, user info  
- **Swap Integration**: Jupiter & Serum via a unified adapter