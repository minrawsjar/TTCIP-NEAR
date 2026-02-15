# 📱 SMS Request Handler

> **Core SMS processing engine — Rust/Axum**

Parses incoming SMS messages from Twilio, routes commands to backend services, manages user wallets, and sends responses. This is the primary entry point for all user interactions.

---

## Architecture

```
Twilio Webhook (POST /sms/incoming)
    │
    ▼
┌─────────────────────────────────────────────────┐
│         SMS REQUEST HANDLER (Rust)               │
│                                                  │
│  ┌──────────┐   ┌────────────┐   ┌───────────┐ │
│  │  Webhook  │──▶│  Command   │──▶│  Backend  │ │
│  │  Parser   │   │  Router    │   │  Clients  │ │
│  └──────────┘   └────────────┘   └───────────┘ │
│                       │                          │
│              ┌────────┼────────┐                 │
│              ▼        ▼        ▼                 │
│         ┌────────┐ ┌──────┐ ┌──────┐            │
│         │  DB    │ │Wallet│ │Twilio│            │
│         │(SQLite/│ │Mgmt  │ │Reply │            │
│         │Postgres│ │      │ │      │            │
│         └────────┘ └──────┘ └──────┘            │
└─────────────────────────────────────────────────┘
```

---

## SMS Commands

| Command | Example | Description |
|---------|---------|-------------|
| `JOIN <name>` | `JOIN alice` | Create wallet + register `alice.ttcip.eth` |
| `BALANCE` | `BALANCE` | Check TXTC + ETH balance |
| `SEND <amount> TXTC TO <recipient>` | `SEND 10 TXTC TO alice.ttcip.eth` | Transfer tokens (via Yellow Network batching) |
| `SWAP <amount> TXTC` | `SWAP 5 TXTC` | Swap TXTC → ETH via Uniswap V3 |
| `REDEEM <code>` | `REDEEM BB673BCC` | Redeem voucher for TXTC + gas ETH |
| `CASHOUT <amount> TXTC` | `CASHOUT 10 TXTC` | Convert TXTC → USDC on Arc via CCTP |
| `BUY <amount>` | `BUY 10` | Buy Lycamobile airtime with TXTC |
| `HELP` | `HELP` | List available commands |

---

## Folder Structure

```
sms-request-handler/
├── Cargo.toml              # Rust dependencies
├── Dockerfile              # Docker build
├── .env                    # Environment variables
├── textchain.db            # SQLite database (dev)
└── src/
    ├── main.rs             # Axum server setup, route mounting
    ├── config.rs           # Environment config loading
    ├── routes.rs           # HTTP route definitions
    ├── admin.rs            # Admin endpoints (wallet management)
    ├── admin_wallet.rs     # Admin wallet operations
    ├── yellow_client.rs    # Yellow Network HTTP client
    ├── commands/
    │   ├── mod.rs          # Module exports
    │   ├── parser.rs       # SMS command parser (JOIN, SEND, SWAP, etc.)
    │   └── redeem_integration.rs  # Voucher redemption logic
    ├── contracts/
    │   ├── mod.rs          # Module exports
    │   ├── config.rs       # Contract addresses config
    │   └── service.rs      # Smart contract interaction client
    ├── db/
    │   ├── mod.rs          # Database pool + migrations
    │   ├── users.rs        # User CRUD (phone → wallet mapping)
    │   ├── deposits.rs     # Deposit tracking
    │   ├── vouchers.rs     # Voucher state management
    │   └── address_book.rs # ENS name → address cache
    ├── sms/
    │   ├── mod.rs          # Module exports
    │   ├── twilio.rs       # Twilio SMS send/receive
    │   └── webhook.rs      # Twilio webhook handler + signature validation
    └── wallet/
        ├── mod.rs          # Module exports
        ├── wallet.rs       # Wallet creation + key management
        ├── provider.rs     # Ethereum RPC provider setup
        ├── chains.rs       # Multi-chain configuration
        ├── tokens.rs       # ERC20 token interactions
        └── aa.rs           # Account Abstraction (ERC-4337) types
```

---

## Tech Stack

| Technology | Purpose |
|------------|---------|
| **Rust** | Systems language for performance + safety |
| **Axum 0.7** | Async web framework |
| **Tokio** | Async runtime |
| **ethers-rs 2.x** | Blockchain interactions |
| **SQLx** | Async database (PostgreSQL + SQLite) |
| **Reqwest** | HTTP client for backend services |
| **Twilio** | SMS send/receive |

---

## Setup

### Prerequisites

- Rust 1.70+
- PostgreSQL 16 (production) or SQLite (development)

### Environment

```env
# Server
SERVER_HOST=0.0.0.0
SERVER_PORT=8080

# Blockchain
PRIVATE_KEY=0x...
RPC_URL=https://eth-sepolia.g.alchemy.com/v2/YOUR_KEY

# Twilio
TWILIO_ACCOUNT_SID=AC...
TWILIO_AUTH_TOKEN=...
TWILIO_PHONE_NUMBER=+18449862896

# Backend services
BACKEND_URL=http://localhost:3000
ARC_SERVICE_URL=http://localhost:8084

# Database
DATABASE_URL=postgres://textchain:textchain@localhost:5432/textchain
```

### Run

```bash
cd sms-request-handler

# Development
cargo run

# Production
cargo build --release
./target/release/textchain
```

### Docker

```bash
docker compose up -d sms-handler
```

---

## Service Communication

```
SMS Handler (8080) ──▶ Backend API (3000)    — redeem, swap, balance, ENS
                   ──▶ Yellow Network (8083) — batched SEND transfers
                   ──▶ Arc Service (8084)    — CASHOUT to USDC
                   ──▶ Airtime Service (8082)— BUY airtime
```

