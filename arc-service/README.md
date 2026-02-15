# 🔵 Arc Service — Circle CCTP Cashout

> **Circle Partner Prize Submission**

SMS-based cashout service that converts TXTC tokens to USDC on Arc Testnet using Circle's Cross-Chain Transfer Protocol (CCTP V2) and Developer-Controlled Wallets.

---

## What We Built

Arc Service enables SMS users to **cash out** their TXTC tokens to USDC on Circle's Arc Testnet — all with a single text message. No browser, no MetaMask, no smartphone required.

```
SMS: "CASHOUT 10 TXTC"
  → Burn TXTC from user wallet
  → Swap TXTC → WETH (Uniswap V3, 0.3% pool)
  → Swap WETH → USDC (Uniswap V3, 0.05% pool)
  → CCTP depositForBurn (Sepolia → Arc, domain 0 → 26)
  → Circle Iris API attestation (~20s Fast Transfer)
  → receiveMessage on Arc → USDC minted to user's Circle Wallet
  → SMS: "Cashout complete! 10 TXTC → ~$38 USDC"
```

---

## Architecture

### Source Files

| File | Purpose |
|------|---------|
| `src/index.ts` | Express API server — wallet, cashout, quote, pay, batch-payout, treasury endpoints |
| `src/cashout-service.ts` | Full cashout pipeline: TXTC → WETH → USDC → CCTP bridge → Arc |
| `src/circle-wallet.ts` | Circle Developer-Controlled Wallets SDK integration |

### Cashout Pipeline

```
┌──────────────────────────────────────────────────────────┐
│                    SEPOLIA (Source)                        │
│                                                          │
│  1. burnFromAny(user, amount)     — Burn TXTC from user  │
│  2. exactInputSingle(TXTC→WETH)  — Uniswap V3 (0.3%)   │
│  3. exactInputSingle(WETH→USDC)  — Uniswap V3 (0.05%)  │
│  4. depositForBurn(USDC)          — CCTP V2 burn         │
└──────────────────────┬───────────────────────────────────┘
                       │ Circle Iris API attestation (~20s)
                       ▼
┌──────────────────────────────────────────────────────────┐
│                  ARC TESTNET (Destination)                │
│                                                          │
│  5. receiveMessage(msg, attestation) — Mint USDC         │
│  6. USDC arrives in user's Circle Wallet                 │
└──────────────────────────────────────────────────────────┘
```

### CCTP V2 Contracts

| Contract | Address | Chain |
|----------|---------|-------|
| **TokenMessengerV2** | `0x8fe6b999dc680ccfdd5bf7eb0974218be2542daa` | Sepolia |
| **MessageTransmitterV2** | `0xe737e5cebeeba77efe34d4aa090756590b1ce275` | Arc Testnet |
| **USDC** | `0x1c7d4b196cb0c7b01d743fbc6116a902379c7238` | Sepolia |

| Parameter | Value |
|-----------|-------|
| Source Domain | `0` (Sepolia) |
| Destination Domain | `26` (Arc Testnet) |
| Max Fee | 1.5 USDC |
| Finality Threshold | `1000` (Fast Transfer) |

---

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/health` | Health check |
| `POST` | `/api/arc/wallet` | Create or get Circle Wallet on Arc |
| `GET` | `/api/arc/balance/:phone` | Get USDC balance on Arc |
| `POST` | `/api/arc/quote` | Get cashout quote (TXTC → USDC estimate) |
| `POST` | `/api/arc/cashout` | Full cashout: TXTC → USDC on Arc |
| `POST` | `/api/arc/pay` | Send USDC to another user on Arc |
| `POST` | `/api/arc/batch-payout` | Multi-recipient USDC payout |
| `GET` | `/api/arc/treasury` | Treasury dashboard data |
| `GET` | `/api/arc/payouts` | Payout history |
| `GET` | `/api/arc/wallets` | List all registered wallets (admin) |

---

## Circle Integration

### Developer-Controlled Wallets

Each SMS user gets a **Circle Developer-Controlled Wallet** on Arc Testnet:

- Wallets are created via Circle's SDK (`@circle-fin/developer-controlled-wallets`)
- Organized in a "TextChain Users" wallet set
- Phone → wallet mapping persisted to `wallets.json`
- Supports balance queries, USDC transfers, and batch payouts

### CCTP V2 Fast Transfer

The bridge uses CCTP V2's **Fast Transfer** mode:

1. **Approve** USDC for TokenMessengerV2
2. **depositForBurn** — burns USDC on Sepolia with `minFinalityThreshold=1000`
3. **Poll Iris API** — `GET /v2/messages/0?transactionHash=<tx>` until `status=complete`
4. **receiveMessage** — mints USDC on Arc Testnet with the attestation

Attestation typically arrives in **~20 seconds** with Fast Transfer.

---

## Setup

### Prerequisites

- Node.js v18+
- Circle Developer Console account
- Alchemy/Infura Sepolia RPC

### Environment Variables

```env
PRIVATE_KEY=0x...                    # Backend wallet (Sepolia + Arc)
SEPOLIA_RPC_URL=https://eth-sepolia.g.alchemy.com/v2/YOUR_KEY
ARC_RPC_URL=https://arc-testnet.drpc.org
CIRCLE_API_KEY=...                   # Circle Developer Console
CIRCLE_ENTITY_SECRET=...            # Circle entity secret (hex-encoded)
TXTC_ADDRESS=0x4d054FB258A260982F0bFab9560340d33D9E698B
WETH_ADDRESS=0xfFf9976782d46CC05630D1f6eBAb18b2324d6B14
ETH_PRICE_USD=2500                   # For USDC estimate display
PORT=8084
```

### Run

```bash
cd arc-service
cp .env.example .env
npm install
npm run dev
```

### Docker

```bash
docker compose up -d arc
```

---

## Tech Stack

| Technology | Purpose |
|------------|---------|
| **Circle CCTP V2** | Cross-chain USDC bridging (Sepolia → Arc) |
| **Circle Developer-Controlled Wallets** | Per-user wallets on Arc Testnet |
| **Circle Iris API** | CCTP attestation polling |
| **Uniswap V3** | TXTC → WETH → USDC swap pipeline |
| **ethers.js v6** | Blockchain interactions |
| **Express** | REST API server |
| **TypeScript** | Type-safe implementation |

