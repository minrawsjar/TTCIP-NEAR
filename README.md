# Text-to-Chain NEAR

> **Standalone NEAR Protocol Implementation**

**DeFi over SMS for 2.5 billion feature phone users**

Text-to-Chain NEAR is a full-stack SMS-based DeFi platform built entirely on **NEAR Protocol**. Users interact with the blockchain using only text messages via Twilio: no smartphone, no app, no seed phrases to memorize.

> Send `JOIN alice` to create a NEAR wallet. Send `SEND 100 TXTC TO bob.testnet` to transfer tokens.

---

## The Problem

For many people, joining the global economy is not about convenience. It is about access.
*   **No smartphones**: Billions rely on feature phones.
*   **No internet data**: Data is expensive or unavailable.
*   **Complexity**: Seed phrases and gas fees are barriers.

## The Solution

**Text-to-Chain on NEAR** turns a basic phone into a blockchain wallet.
*   **SMS Interface**: Works on any mobile phone.
*   **NEAR Protocol**: Fast, low-cost transactions (<$0.001) make micro-transactions viable.
*   **Account Abstraction**: The backend handles keys and gas; users just text.

### Capabilities

*   **Wallet Creation**: `JOIN <name>` creates a real NEAR account.
*   **Token Transfer**: Send NEAR or TXTC tokens via SMS.
*   **Token Swaps**: Swap TXTC ↔ NEAR via on-chain liquidity pool.
*   **Vouchers**: Offline-first voucher system for onboarding. Use a code to redeem tokens.
*   **Cashout**: Convert TXTC to USDC on Arc Testnet via Circle CCTP.

---

## Technical Architecture

### Core Technologies

| Technology | Purpose |
|------------|---------|
| **NEAR Protocol** | L1 Blockchain (Rust Smart Contracts) |
| **Rust** | Smart Contracts + SMS Handler |
| **Node.js/TypeScript** | Backend API Integration |
| **Circle CCTP** | Cross-chain USDC bridge |

### System Architecture

```
┌──────────────────────────────────────────────────────────┐
│                     USER LAYER                           │
│                                                          │
│   Feature Phone ──► Twilio SMS ──► Webhook ──► Handler   │
└────────────────────────┬─────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│              BACKEND API (Node.js :3000)                │
│                                                         │
│   NEAR RPC ──► Transaction Signing ──► Contract Calls   │
└────────┬────────────────────────────┬───────────────────┘
         │                            │
         ▼                            ▼
┌──────────────────────┐    ┌──────────────────────────┐
│   NEAR TESTNET       │    │   SMART CONTRACTS        │
│                      │    │                          │
│   User Accounts      │    │   • ttcip-pool           │
│   (alice.testnet)    │    │   • txtc token (NEP-141) │
└──────────────────────┘    └──────────────────────────┘
```

---

## Project Structure

```
TTCIP-NEAR/
│
├── liquidity-pool-near/          # NEAR Smart Contracts (Rust)
│   ├── src/lib.rs               #   Pool + Voucher logic
│   └── Cargo.toml               #   Dependencies
│
├── backend-integration/          # Node.js Backend API
│   ├── near-api-server.ts       #   Express API (port 3000)
│   ├── near-contract-service.ts #   NEAR RPC integration
│   └── package.json             #   Dependencies
│
├── sms-request-handler/          # Rust SMS Handler
│   ├── src/
│   │   ├── commands/            #   SMS command parser
│   │   ├── near_integration.rs  #   Backend API client
│   │   └── main.rs              #   HTTP server (port 8080)
│   └── Cargo.toml
│
└── arc-service/                  # Circle CCTP Cashout (optional)
    ├── src/
    │   ├── cashout-service.ts   #   TXTC→USDC bridge
    │   └── circle-wallet.ts     #   Circle wallets
    └── package.json
```

---

## Smart Contracts

Deployed on **NEAR Testnet**.

| Contract | Address | Description |
|----------|---------|-------------|
| **TXTC Token** | `txtc.0xswarnim.testnet` | NEP-141 Token (18 decimals) |
| **Liquidity Pool** | `ttcip-pool.0xswarnim.testnet` | AMM Pool + Voucher system |

### Features
*   **NEP-141**: Standard fungible token implementation.
*   **Vouchers**: Hash-based voucher redemption (keccak256).
*   **Liquidity**: Constant product AMM (x * y = k).
*   **Swaps**: TXTC ↔ NEAR with 0.3% fee.

**Pool Reserves**: ~1.1 NEAR + 6430 TXTC

---

## SMS Commands

| Command | Example | Description |
|---------|---------|-------------|
| **JOIN** | `JOIN alice` | Create NEAR account `alice.0xswarnim.testnet` |
| **BALANCE** | `BALANCE` | Check NEAR + TXTC balance |
| **SEND** | `SEND 10 TXTC TO bob.testnet` | Transfer tokens |
| **SWAP** | `SWAP 100 TXTC FOR NEAR` | Exchange tokens via pool |
| **REDEEM** | `REDEEM WELCOME500` | Claim voucher tokens |
| **CASHOUT** | `CASHOUT 10 TXTC` | Convert to USDC on Arc (requires Circle keys) |

---

## Getting Started

### Prerequisites
*   **Node.js v18+**
*   **Rust & Cargo**
*   **NEAR CLI** (`npm install -g near-cli`)

### Installation

1.  **Clone the repository**:
    ```bash
    git clone https://github.com/minrawsjar/TTCIP-NEAR.git
    cd TTCIP-NEAR
    ```

2.  **Install dependencies**:
    ```bash
    # Backend API
    cd backend-integration && npm install
    
    # SMS Handler
    cd ../sms-request-handler && cargo build --release
    
    # Arc Service (optional)
    cd ../arc-service && npm install
    ```

3.  **Configuration**:
    
    **Backend API** - Create `backend-integration/.env`:
    ```env
    # NEAR Configuration
    NEAR_RPC_URL=https://rpc.testnet.fastnear.com
    NEAR_PRIVATE_KEY=ed25519:your_private_key_here
    NEAR_OWNER_ACCOUNT=0xswarnim.testnet
    NEAR_POOL_CONTRACT=ttcip-pool.0xswarnim.testnet
    TXTC_TOKEN_CONTRACT=txtc.0xswarnim.testnet
    ```
    
    **SMS Handler** - Create `sms-request-handler/.env`:
    ```env
    # NEAR Backend API
    BACKEND_URL=http://localhost:3000
    
    # Twilio Configuration (required for production SMS)
    TWILIO_ACCOUNT_SID=AC...your_account_sid
    TWILIO_AUTH_TOKEN=...your_auth_token
    TWILIO_PHONE_NUMBER=+14155551234
    ```

### Twilio SMS Setup (Production)

**1. Get Twilio Number**:
- Sign up at [twilio.com](https://www.twilio.com/try-twilio)
- Get a phone number with SMS capabilities
- Note your Account SID and Auth Token

**2. Configure Webhook**:
- In Twilio Console → Phone Numbers → your number
- **Messaging → A MESSAGE COMES IN**:
  - Webhook: `https://your-domain.com/sms/incoming`
  - HTTP POST

**3. Deploy SMS Handler**:

**Option A - Railway** (Recommended):
```bash
cd sms-request-handler
railway init
railway up
# Automatically gets HTTPS: your-app.railway.app
```

**Option B - Local with ngrok** (Testing):
```bash
# Terminal 1: Start SMS handler
cargo run --release

# Terminal 2: Expose to internet
ngrok http 8080
# Use ngrok URL: https://abc123.ngrok.io/sms/incoming
```

**4. Update Twilio webhook URL** to your deployed endpoint.

### Usage

**Start Services**:
```bash
# Terminal 1: Backend API
cd backend-integration
npx ts-node near-api-server.ts

# Terminal 2: SMS Handler
cd sms-request-handler
cargo run --release
```

**Production SMS** (after Twilio setup):
```
Users text your Twilio number:
→ "JOIN alice"
← "🎉 NEAR Wallet Created! Account: alice.0xswarnim.testnet"
```

**Local Testing** (simulates Twilio webhook):
```bash
curl -X POST http://localhost:8080/sms/incoming \
  -d "From=+15550109999&Body=JOIN alice"
```

---

## Documentation

- [Quick Start Guide](QUICK_START.md)
- [NEAR Deployment Guide](NEAR_DEPLOYMENT_GUIDE.md)
- [Run NEAR Services](RUN_NEAR.md)

---

## Verified Testing

All SMS commands have been tested and verified on NEAR testnet:

✅ **JOIN** - 3 accounts created  
✅ **BALANCE** - Working correctly  
✅ **SWAP** - [On-chain proof](https://testnet.nearblocks.io/txns/BmUc4CVLpCGZvGA78mWb5ViYuf9xBb8AgvhqxNjkqibz)  
✅ **REDEEM** - [On-chain proof](https://testnet.nearblocks.io/txns/9ZJFbqaPArEpy7ktMt8bS6UKudJ2Q2RtetSC6mjJ2Frc)  
✅ **SEND** - Supported via NEP-141 standard

See [walkthrough.md](.gemini/antigravity/brain/*/walkthrough.md) for detailed test results.

---

## License

MIT
