#!/bin/bash
# Run and deploy NEAR stack - Text-to-Chain

set -e

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "🚀 Text-to-Chain NEAR Stack"
echo "=========================="

# 1. Check prerequisites
echo ""
echo "📋 Checking prerequisites..."

if ! command -v near &> /dev/null; then
    echo "❌ NEAR CLI not found. Install: npm install -g near-cli"
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    echo "❌ Rust/Cargo not found. Install: https://rustup.rs"
    exit 1
fi

# 2. Load env
if [ -f "$ROOT/backend-integration/.env" ]; then
    export $(grep -v '^#' "$ROOT/backend-integration/.env" | xargs)
else
    echo "⚠️  No .env found. Copy backend-integration/.env.near.example to backend-integration/.env"
    echo "   Add NEAR_OWNER_ACCOUNT (your testnet account from: near login)"
    exit 1
fi

if [ -z "$NEAR_OWNER_ACCOUNT" ]; then
    echo "❌ Set NEAR_OWNER_ACCOUNT in .env (e.g. yourname.testnet)"
    exit 1
fi

# 3. Deploy pool
echo ""
echo "📦 Deploying TTCIP Pool..."
cd "$ROOT/backend-integration"
node deploy-near-pool.js

# 4. Start backend
echo ""
echo "🌐 Starting NEAR backend (Ctrl+C to stop)..."
npm run start:near &

BACKEND_PID=$!
sleep 3

# 5. Start SMS handler (in another terminal suggestion)
echo ""
echo "📱 To start SMS handler, run in a new terminal:"
echo "   cd $ROOT/sms-request-handler"
echo "   BACKEND_URL=http://localhost:3000 cargo run"
echo ""
echo "Backend running (PID $BACKEND_PID). Press Ctrl+C to stop."

wait $BACKEND_PID
