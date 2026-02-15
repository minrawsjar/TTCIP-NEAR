# Circle — Build Global Payouts and Treasury Systems with USDC on Arc

> **Track:** Build Global Payouts and Treasury Systems with USDC on Arc — $2,500

---

## What We Built

Text-to-Chain enables **SMS-based USDC cashout and global payouts** using Circle's infrastructure. Feature phone users text `CASHOUT 10 TXTC` and receive USDC on Arc Testnet — no smartphone, no browser, no MetaMask.

```
SMS: "CASHOUT 10 TXTC"
  → Burn TXTC from user wallet (Sepolia)
  → Swap TXTC → WETH (Uniswap V3, 1% pool)
  → Swap WETH → USDC (Uniswap V3, 0.05% pool)
  → CCTP V2 depositForBurn (Sepolia → Arc, domain 0 → 26)
  → Circle Iris API attestation (~20s Fast Transfer)
  → receiveMessage on Arc → USDC minted to user's Circle Wallet
  → SMS: "Cashout complete! 10 TXTC → ~$38 USDC"
```

---

## Circle Tools Used

### 1. Arc Testnet (USDC Destination Chain)
- All cashout USDC lands on **Arc Testnet**
- Users receive native USDC on Arc via CCTP bridging
- Arc serves as the settlement layer for real-world value

### 2. USDC (Core Asset)
- USDC is the cashout currency — users convert TXTC → USDC
- Native USDC on both Sepolia (source) and Arc (destination)
- Sepolia USDC: `0x1c7d4b196cb0c7b01d743fbc6116a902379c7238`

### 3. Circle Developer-Controlled Wallets
- Each SMS user gets a **Circle Developer-Controlled Wallet** on Arc Testnet
- Wallets created via `@circle-fin/developer-controlled-wallets` SDK
- Organized in a "TextChain Users" wallet set
- Phone → wallet mapping persisted to disk (survives restarts)
- Supports balance queries, USDC transfers, and batch payouts

### 4. CCTP V2 (Cross-Chain Transfer Protocol)
- **depositForBurn** on Sepolia burns USDC with `minFinalityThreshold=1000` (Fast Transfer)
- **Circle Iris API** polled for attestation (`GET /v2/messages/0?transactionHash=<tx>`)
- **receiveMessage** on Arc mints USDC to user's Circle Wallet
- Attestation arrives in ~20 seconds with Fast Transfer mode

---

## How We Meet the Criteria

### Automated Payout Logic
- SMS command `CASHOUT` triggers a fully automated pipeline
- No human intervention: burn → swap → bridge → mint → SMS notification
- Async processing: user gets immediate SMS acknowledgment, cashout completes in background

### Multi-Recipient Settlement
- **Batch Payout endpoint** (`POST /api/arc/batch-payout`): send USDC to multiple recipients in one call
- **Pay endpoint** (`POST /api/arc/pay`): USDC transfers between Circle Wallets on Arc
- Treasury dashboard (`GET /api/arc/treasury`) shows aggregate balances and payout history

### Policy-Based Payouts
- Conversion rates are configurable via environment variables
- Min/max transaction limits enforced
- Phone → wallet mapping ensures identity verification via SMS carrier

---

## Architecture Diagram

```
┌──────────────────────────────────────────────────────────┐
│                    SEPOLIA (Source)                        │
│                                                          │
│  1. burnFromAny(user, amount)     — Burn TXTC from user  │
│  2. exactInputSingle(TXTC→WETH)  — Uniswap V3 (1%)     │
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

---

## CCTP V2 Contract Addresses

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
| `POST` | `/api/arc/wallet` | Create or get Circle Wallet on Arc |
| `GET` | `/api/arc/balance/:phone` | Get USDC balance on Arc |
| `POST` | `/api/arc/quote` | Get cashout quote (TXTC → USDC estimate) |
| `POST` | `/api/arc/cashout` | Full cashout: TXTC → USDC on Arc |
| `POST` | `/api/arc/pay` | Send USDC to another user on Arc |
| `POST` | `/api/arc/batch-payout` | Multi-recipient USDC payout |
| `GET` | `/api/arc/treasury` | Treasury dashboard data |

---

## Source Code

| File | Purpose |
|------|---------|
| [`arc-service/src/index.ts`](../arc-service/src/index.ts) | Express API — wallet, cashout, pay, batch-payout, treasury |
| [`arc-service/src/cashout-service.ts`](../arc-service/src/cashout-service.ts) | Full pipeline: TXTC → WETH → USDC → CCTP → Arc |
| [`arc-service/src/circle-wallet.ts`](../arc-service/src/circle-wallet.ts) | Circle Developer-Controlled Wallets SDK |

---

## Product Feedback for Circle

### What Worked Well
- **Developer-Controlled Wallets SDK** is excellent for server-side wallet management — perfect for our SMS use case where users can't sign transactions
- **CCTP V2 Fast Transfer** (~20s attestation) makes SMS cashout feel responsive
- **Iris API** is reliable and well-documented for attestation polling

### Suggestions
- **Arc Testnet faucet** could be more generous — we ran out of testnet USDC during development
- **SDK TypeScript types** for `ARC-TESTNET` blockchain enum aren't fully typed yet (had to cast as `any`)
- **Webhook support** for CCTP attestation completion would eliminate polling and reduce latency
- **Batch depositForBurn** — a single transaction that burns USDC for multiple recipients would dramatically reduce gas costs for our payout use case

---

## Future Plans with Circle

1. **Mainnet deployment** — Arc mainnet for real USDC payouts to SMS users in developing countries
2. **Circle Gateway** — enable fiat on/off ramp so users can cash out USDC to mobile money (M-Pesa, MTN MoMo)
3. **Bridge Kit** — integrate for a smoother multi-chain USDC experience
4. **Programmable Wallets** — use Circle's smart contract wallets for gasless USDC transfers on Arc
5. **Revenue distribution** — automated USDC payouts to voucher shop owners (2% commission)

---

## Why Text-to-Chain + Circle

**2.5 billion feature phone users** are excluded from crypto because they can't install apps or use browser wallets. Text-to-Chain bridges this gap with SMS, and Circle's infrastructure provides the **real-world value layer**:

- **USDC** = stable, trusted, globally recognized
- **Arc** = fast, low-cost settlement
- **CCTP** = seamless cross-chain movement
- **Developer-Controlled Wallets** = perfect for server-managed SMS wallets

Together, we make global payouts accessible to anyone with a phone number.
