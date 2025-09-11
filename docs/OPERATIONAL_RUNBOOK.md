# Operational Runbook

## 1. Overview
Defines procedures for deploying, monitoring, and maintaining the Solana Tax & Reward program and its clients.

## 2. Pre-Deployment Checks
- Ensure CI/CD pipelines are green.
- Confirm program tests pass (`anchor test`).
- Validate config values (tax rates, pause flag).
- Backup on-chain state or snapshot devnet.

## 3. Deployment Steps
1. Run `scripts/deploy/deploy.sh` with correct cluster and wallet.
2. Confirm new program ID in `Anchor.toml`.
3. Redeploy clients:  
   - `clients/dapp`: `npm run build && npm run start`  
   - `clients/batch-service`: `npm run build && npm run start`
4. Verify on-chain logs for successful SWAP and CONFIG updates.

## 4. Post-Deployment Validation
- Check Prometheus metrics: swap rates, reward distributions.
- Run test transactions on devnet/mainnet-fork.
- Validate user reward pulls from UI.

## 5. Rollback Procedure
- Pause program via multisig pause flag.
- Deploy previous BPF build via `scripts/deploy/rollback.sh`.
- Restart clients on last known stable version.
- Unpause after verification.

## 6. Alerts & Escalation
- Alert on high slippage or failed swaps.
- Monitor RPC errors and retry spikes.
- Escalate to on-call on critical failures.

## 7. Maintenance Tasks
- Cleanup stale `UserInfo` accounts weekly.
- Rotate multisig keys bi-annually.
- Review logs and update runbook as needed.