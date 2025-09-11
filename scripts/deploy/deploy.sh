#!/usr/bin/env bash
set -euo pipefail

# Usage: ./deploy.sh <cluster> <wallet>
# Example: ./deploy.sh mainnet ~/.config/solana/id.json

CLUSTER=${1:-"localnet"}
WALLET=${2:-"${HOME}/.config/solana/id.json"}

echo "🚀 Building program..."
anchor build

echo "🛰️ Deploying to $CLUSTER with wallet $WALLET..."
anchor deploy --provider.cluster $CLUSTER --provider.wallet $WALLET

echo "✅ Deployment complete. Update Anchor.toml with the new program ID."