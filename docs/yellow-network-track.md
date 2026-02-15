# Yellow Network — Integrate Yellow SDK

> **Track:** Integrate Yellow SDK: Trading apps / Prediction markets / Marketplaces — $15,000

---

## What We Built

Text-to-Chain uses the **Yellow SDK (Nitrolite protocol)** to batch SMS payment transfers off-chain, settling on-chain only when needed. Users text `SEND 10 TXTC TO alice.ttcip.eth` and the transfer happens **instantly via Yellow Network state channels** — zero gas, zero delay.

```
SMS: "SEND 10 TXTC TO alice.ttcip.eth"
  → SMS Handler parses command
  → Transaction queued in Yellow Batch Service
  → Off-chain transfer via Nitrolite state channel (instant)
  → On-chain TXTC mint to recipient when session closes
  → SMS: "✅ 10 TXTC sent to alice.ttcip.eth"
```

---

## Yellow SDK Integration

### Nitrolite SDK Usage

We use `@erc7824/nitrolite` (v0.5.3) with the following SDK functions:

| SDK Function | Where Used | Purpose |
|-------------|------------|---------|
| `createAuthRequestMessage` | `batch-service.ts` | Initiate EIP-712 auth challenge |
| `createEIP712AuthMessageSigner` | `batch-service.ts` | Sign auth challenge |
| `createAuthVerifyMessageFromChallenge` | `batch-service.ts` | Complete auth handshake |
| `createTransferMessage` | `batch-service.ts` | Execute off-chain transfers |
| `createGetConfigMessage` | `index.ts` | Fetch clearnode configuration |
| `createGetLedgerBalancesMessage` | `batch-service.ts` | Check off-chain balances |
| `createCreateChannelMessage` | `index.ts` | Open state channels |
| `createResizeChannelMessage` | `index.ts` | Resize channel capacity |
| `createCloseChannelMessage` | `index.ts` | Close and settle on-chain |
| `NitroliteClient` | `index.ts` | Core client for on-chain operations |
| `WalletStateSigner` | `index.ts` | Sign state updates |
| `createECDSAMessageSigner` | `index.ts` | ECDSA message signing |

### Connection Details

| Parameter | Value |
|-----------|-------|
| **WebSocket** | `wss://clearnet-sandbox.yellow.com/ws` |
| **Custody Address** | `0x019B65A265EB3363822f2752141b3dF16131b262` |
| **Adjudicator** | `0x7c7ccbc98469190849BCC6c926307794fDfB11F2` |
| **Asset** | `ytest.usd` (sandbox) |
| **Network** | Sepolia Testnet |

---

## Off-Chain Transaction Logic

### Batch Processing (3-Minute Sessions)

```
┌─────────────────────────────────────────────────┐
│         YELLOW BATCH SERVICE                     │
│                                                  │
│  ┌─────────────────────────────────────────┐    │
│  │  Transaction Queue (In-Memory)           │    │
│  │  • +919876543210: 10 TXTC → alice        │    │
│  │  • +918595057429: 5 TXTC → bob           │    │
│  │  • +917766554433: 20 TXTC → charlie      │    │
│  └─────────────────────────────────────────┘    │
│                    │                             │
│                    ▼ Every 3 minutes             │
│  ┌─────────────────────────────────────────┐    │
│  │  Batch Processor                         │    │
│  │  1. Connect to clearnode WebSocket       │    │
│  │  2. Auth (EIP-712 challenge-response)    │    │
│  │  3. Send N transfers off-chain           │    │
│  │  4. Mint TXTC to recipients on-chain     │    │
│  │  5. Send SMS notifications               │    │
│  │  6. Disconnect                           │    │
│  └─────────────────────────────────────────┘    │
└─────────────────────────────────────────────────┘
```

### How It Works

1. **User sends SMS** → `SEND 10 TXTC TO alice.ttcip.eth`
2. **SMS Handler** resolves ENS name, queues transaction via `POST /api/yellow/send`
3. **Yellow Batch Service** holds transaction in memory queue
4. **Every 3 minutes**, batch processor:
   - Opens WebSocket to `wss://clearnet-sandbox.yellow.com/ws`
   - Authenticates via EIP-712 challenge-response
   - Executes all queued transfers off-chain via `createTransferMessage`
   - Mints TXTC tokens to recipients on Sepolia
   - Sends SMS confirmation to all participants
   - Disconnects

### On-Chain Settlement

When a batch session closes, TXTC tokens are minted on-chain to recipients. The Yellow Network state channel ensures all off-chain transfers are cryptographically valid before settlement.

---

## Cost Savings

```
Traditional Approach (10 SMS payments):
┌─────────────────────────────────────────┐
│ Payment 1: On-chain transfer = 0.003 ETH│
│ Payment 2: On-chain transfer = 0.003 ETH│
│ ... (8 more payments)                    │
│ Total: 0.03 ETH (~$90)                  │
└─────────────────────────────────────────┘

Yellow Network Batch (10 SMS payments):
┌─────────────────────────────────────────┐
│ 10 off-chain transfers      = 0 ETH     │
│ 1 batch mint on-chain       = 0.003 ETH │
│ Total: 0.003 ETH (~$9)                  │
│ Savings: $81 (90%)                       │
└─────────────────────────────────────────┘
```

---

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/yellow/send` | Queue a transaction for next batch |
| `GET` | `/api/yellow/status` | Service status + pending count |
| `GET` | `/api/yellow/pending` | List pending transactions |
| `GET` | `/health` | Health check |

---

## Source Code

| File | Purpose |
|------|---------|
| [`yellow/src/batch-service.ts`](../yellow/src/batch-service.ts) | Core batch processor — queue, auth, transfer, settle |
| [`yellow/src/api-server.ts`](../yellow/src/api-server.ts) | Express API for queuing transactions |
| [`yellow/src/index.ts`](../yellow/src/index.ts) | Interactive CLI — channel create, resize, transfer, close |
| [`yellow/src/batch-server.ts`](../yellow/src/batch-server.ts) | Dev server entry point |

---

## Why Yellow Network for SMS Payments

### The Problem
Traditional blockchain transfers are **prohibitively expensive** for feature phone users in developing countries:
- Each SMS payment requires an on-chain transaction
- Gas costs: ~0.003 ETH (~$9) per transfer
- Wait time: 15+ seconds for confirmation
- **Result:** Unusable for micro-payments via SMS

### The Yellow Network Solution
- **Instant**: Off-chain transfers complete in milliseconds
- **Free**: No gas for individual transfers
- **Batched**: Single on-chain settlement for unlimited transfers
- **Session-based**: 3-minute batch windows match SMS interaction patterns perfectly

### Why It's the Best Fit
SMS payments are inherently **bursty and micro** — many small transfers from many users. Yellow Network's state channel model is ideal because:
1. Users don't need to manage channels (backend handles everything)
2. Batch processing aligns with SMS's async nature
3. Cost savings scale with volume — more users = more savings
4. No wallet software needed on the user's device

---

## Demo Flow (2-3 Minutes)

1. **User A** texts `SEND 10 TXTC TO alice.ttcip.eth` → instant SMS: "Transfer queued!"
2. **User B** texts `SEND 5 TXTC TO bob.ttcip.eth` → instant SMS: "Transfer queued!"
3. **3 minutes later** → Yellow batch processes both transfers off-chain
4. **Both users** get SMS: "✅ Transfer complete!"
5. **On-chain**: single batch mint settles both transfers

---

## Future Plans with Yellow Network

1. **Multi-token support** — batch USDC, USDT transfers alongside TXTC
2. **Dynamic batch sizing** — adjust batch windows based on queue depth (1 min for high volume, 5 min for low)
3. **Cross-chain batching** — use Yellow's multi-chain support to batch transfers across Sepolia, Polygon, Base
4. **Mainnet deployment** — production Yellow Network for real-value SMS payments
5. **Micropayment channels** — enable per-message tipping (0.01 TXTC per SMS interaction)
6. **Agent-driven batching** — AI agent that optimizes batch timing based on gas prices and queue depth
