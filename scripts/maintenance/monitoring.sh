#!/usr/bin/env bash
set -euo pipefail

# Usage: ./monitoring.sh <rpc_url> <metrics_port>
# Example: ./monitoring.sh https://api.mainnet-beta.solana.com 9100

RPC_URL=${1:-"http://127.0.0.1:8899"}
PORT=${2:-9100}

echo "Starting Solana Prometheus exporter..."
solana-prom --rpc-url "$RPC_URL" --metrics-port "$PORT" &

echo "Prometheus exporter running on port $PORT."
echo "Configure Prometheus and Alertmanager to scrape metrics and alert on high slippage or failed swaps."