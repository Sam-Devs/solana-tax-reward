#!/usr/bin/env bash
# Compute Benchmark: simulate taxed_swap_and_distribute and capture compute units consumed.

# Load environment variables or defaults
RPC_URL=${SOLANA_RPC_URL:-http://localhost:8899}
PROGRAM_ID=$(grep 'solana_tax_reward' Anchor.toml | sed -n "s/.*= \"\\(.*\\)\"/\\1/p")

if [ -z "$PROGRAM_ID" ]; then
  echo "Error: PROGRAM_ID not found in Anchor.toml"
  exit 1
fi

# Build and deploy to localnet
anchor build
anchor deploy --provider.cluster localnet

# Create a keypair for testing
KEYPAIR=test-keypair.json
solana-keygen new --no-passphrase -o $KEYPAIR

# Airdrop some SOL for fees
solana airdrop 2 --url $RPC_URL --keypair $KEYPAIR

# Simulate a swap instruction
TX_LOG=$(anchor run simulate-swap --provider.cluster localnet -- --amount 1 --min-out 1 2>&1)

# Extract compute unit logs
echo "$TX_LOG" | grep -E "Compute units consumed"
exit 0