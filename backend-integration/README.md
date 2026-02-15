# 🔧 Backend Integration (NEAR)

> **TypeScript/Express API server — Port 3000**

Central API layer that bridges the SMS handler with NEAR smart contracts. Handles account creation, balance queries, token swaps, voucher redemption, and transaction signing.

---

## Architecture

```
SMS Handler (Rust :8080) ──▶ Backend API (Express :3000)
                                    │
                        ┌───────────┼───────────┐
                        ▼           ▼           ▼
                 ┌────────────┐ ┌─────────┐ ┌──────────┐
                 │  Contract  │ │  NEAR   │ │  Account │
                 │  Service   │ │   RPC   │ │  Manager │
                 │ (swap,     │ │(balance,│ │ (create, │
                 │  redeem)   │ │  call)  │ │  sign)   │
                 └──────┬─────┘ └────┬────┘ └────┬─────┘
                        │            │           │
                        ▼            ▼           ▼
              ┌─────────────────────────────────────────┐
              │           NEAR Testnet                  │
              │  ttcip-pool · txtc token · User Accounts│
              └─────────────────────────────────────────┘
```

---

## Files

```
backend-integration/
├── near-api-server.ts       # Express API — all endpoints (port 3000)
├── near-contract-service.ts # NEAR contract interactions
├── near.config.ts           # NEAR RPC configuration
├── package.json             # Dependencies
├── tsconfig.json            # TypeScript config
└── .env                     # Environment variables
```

**Test files removed**: `test-*.js`, `deploy-*.js` — use NEAR CLI for deployments.

---

## API Endpoints

### Core

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/health` | Health check |
| `POST` | `/api/join` | Create NEAR account |
| `GET` | `/api/balance/:phone` | Get NEAR + TXTC balance |
| `POST` | `/api/send` | Send TXTC to account |
| `POST` | `/api/swap` | Swap TXTC ↔ NEAR |
| `POST` | `/api/redeem` | Redeem voucher for TXTC |

### Examples

**Create Account**:
```bash
curl -X POST http://localhost:3000/api/join \
  -H "Content-Type: application/json" \
  -d '{"phoneNumber":"+15550109999","desiredName":"alice"}'
```

**Get Balance**:
```bash
curl http://localhost:3000/api/balance/+15550109999
```

**Swap TXTC for NEAR**:
```bash
curl -X POST http://localhost:3000/api/swap \
  -H "Content-Type: application/json" \
  -d '{"userAddress":"alice.0xswarnim.testnet","amount":"100","tokenType":"TXTC"}'
```

**Redeem Voucher**:
```bash
curl -X POST http://localhost:3000/api/redeem \
  -H "Content-Type: application/json" \
  -d '{"voucherCode":"WELCOME500","userAddress":"alice.0xswarnim.testnet"}'
```

---

## Deployed Contracts (NEAR Testnet)

| Contract | Address |
|----------|---------|
| **TXTC Token** | `txtc.0xswarnim.testnet` |
| **Liquidity Pool** | `ttcip-pool.0xswarnim.testnet` |

---

## Setup

```bash
cd backend-integration
npm install
```

### Environment

```env
# NEAR Configuration
NEAR_RPC_URL=https://rpc.testnet.fastnear.com
NEAR_PRIVATE_KEY=ed25519:your_private_key_here
NEAR_OWNER_ACCOUNT=0xswarnim.testnet
NEAR_POOL_CONTRACT=ttcip-pool.0xswarnim.testnet
TXTC_TOKEN_CONTRACT=txtc.0xswarnim.testnet
TXTC_DECIMALS=18
```

### Run

```bash
# Development
npx ts-node near-api-server.ts

# Production (PM2)
pm2 start near-api-server.ts --interpreter=ts-node
```

---

## Integration with SMS Handler

The Rust SMS handler (`sms-request-handler`) calls this API:

```rust
// SMS: "SWAP 100 TXTC FOR NEAR"
let response = client.post("http://localhost:3000/api/swap")
    .json(&json!({
        "userAddress": "alice.0xswarnim.testnet",
        "amount": "100",
        "tokenType": "TXTC"
    }))
    .send().await?;
```

---

## Tech Stack

| Technology | Purpose |
|------------|---------|
| **Express** | REST API server |
| **near-api-js** | NEAR blockchain interactions |
| **TypeScript** | Type-safe implementation |

---

## Next Steps

- Add WebSocket for real-time transaction updates
- Implement transaction retry logic
- Add rate limiting for API endpoints
