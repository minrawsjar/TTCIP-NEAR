# Deep Technical Overview

A detailed technical description of the Text-to-Chain platform: architecture, onboarding, features, data flows, and trust model.

---

## 1. Platform Summary

Text-to-Chain is an **SMS-based DeFi platform** that lets users create wallets, hold and send tokens, swap, bridge cross-chain, and cash out to stablecoins using only text messages. The stack is built so that **feature-phone users** never need a smartphone, browser, or MetaMask.

**Core technical choices:**

- **SMS as the only client** — Twilio (or similar) receives SMS, forwards to our webhook; responses go back via SMS.
- **Server-side command handling** — A Rust service parses commands, looks up users, and calls backend microservices.
- **Phone → wallet binding** — One wallet per phone; the server maps phone numbers to wallet addresses and holds key material needed to sign on behalf of users.
- **On-chain settlement** — User funds live in their own addresses on public chains (e.g. Sepolia, Arc); swaps, bridges, and cashouts execute via smart contracts and third-party protocols (Uniswap, Yellow).

---

## 2. Architecture (High Level)

```
User (SMS) → Twilio → Cloudflare Tunnel → SMS Request Handler (Rust :8080)
                                                    │
                    ┌───────────────────────────────┼───────────────────────────────┐
                    ▼                               ▼                               ▼
            Backend API (:3000)            Yellow Batch (:8083)            Arc/CCTP (:8084)
            (redeem, balance, swap,        (batched SEND via              (CASHOUT TXTC→USDC,
             bridge, ENS, notify)           Nitrolite)                     Circle Wallets)
                    │                               │                               │
                    └───────────────────────────────┴───────────────────────────────┘
                                                    │
                                    Sepolia / Arc / Li.Fi chains
```

- **SMS Request Handler** — Single entrypoint for all SMS. Parses commands, resolves user from phone, reads/writes DB, and calls the right backend service.
- **Backend API** — Contract interactions (redeem, balance, swap), ENS registration, Li.Fi bridge, deposit monitoring, SMS notifications.
- **Yellow** — Off-chain batching for `SEND`; periodic on-chain settlement so TXTC ends up in recipients’ wallets.
- **Arc service** — CASHOUT: TXTC → WETH → USDC on Sepolia, then CCTP to Arc; Circle Developer-Controlled Wallets per user; batch payouts and treasury APIs.

Data stores:

- **SMS handler** — SQLite (or Postgres): users (phone, wallet_address, encrypted_private_key, pin_hash, ens_name), vouchers, deposits, address_book.
- **Arc service** — Persistent wallet mapping (e.g. `wallets.json` or DB) for Circle wallets keyed by phone.

---

## 3. Onboarding: How Users Get On Chain

### 3.1 JOIN Flow

1. User sends **`JOIN`** or **`JOIN alice`** (optional ENS name).
2. SMS handler receives the webhook (Twilio), extracts `From` (phone) and `Body`.
3. **Command parse** — Parser recognizes `Join { ens_name }`.
4. **User lookup** — If user already exists for this phone, they get a “already registered” style message; optionally prompted to pick a name if not yet set.
5. **New user:**
   - **Wallet creation** — Server generates a new ECDSA key pair (e.g. `Wallet::new(&mut OsRng)`), derives address.
   - **Persistence** — `wallet_address` and `encrypted_private_key` (key material, hex-encoded) are stored in the DB, keyed by phone.
   - User is told their wallet is created and is prompted to choose a name: **`JOIN <name>`**.
6. **ENS (if name provided):**
   - Name is validated (length, alphanumeric).
   - Backend is called to register the ENS subdomain (e.g. `alice.ttcip.eth`) and link it to the user’s wallet address.
   - On success, user gets a confirmation with their name and deposit address.

Result: **one wallet per phone**, **one ENS subdomain per user** (optional), and the server can sign transactions for that user when they send commands.

### 3.2 First Use After JOIN

- **DEPOSIT** — User gets their wallet address to receive funds.
- **REDEEM &lt;code&gt;** — Voucher redemption mints TXTC (and optionally ETH) to their wallet; no prior shop registration.
- **BALANCE** — Reads from chain (and/or backend) to show TXTC and native token balance.

Onboarding is **zero-install**: no app, no seed phrase shown over SMS (by design), no gas UX. The tradeoff is that **key material lives on the server** (see Trust model below).

---

## 4. Features and How They Work

### 4.1 Wallet and Identity

- **Wallet** — EOA created on first JOIN; address and key material stored server-side.
- **ENS** — Subdomain under a shared parent (e.g. `*.ttcip.eth`); registered via backend (commit–reveal or equivalent); used for human-readable `SEND` (e.g. `SEND 10 TXTC TO alice.ttcip.eth`).
- **PIN** — Optional PIN stored as hash; can gate sensitive operations (implementation may vary).

### 4.2 SEND (Yellow Network)

- User: **`SEND 10 TXTC TO alice.ttcip.eth`** (or address).
- Handler resolves recipient (ENS → address or contact), queues the transfer with the Yellow service.
- **Yellow** uses Nitrolite (state channels): off-chain transfers, batched settlement every few minutes; on-chain result is TXTC minted to recipient on Sepolia.
- User gets an SMS when the batch is settled.

### 4.3 SWAP

- **`SWAP 5 TXTC`** — Swap TXTC for ETH (or native token).
- Handler calls backend; backend burns/moves user’s TXTC, executes swap via **Uniswap V3** (TXTC/WETH pool), sends native token to user’s wallet.
- Async; SMS notification on success/failure.

### 4.4 BRIDGE (Li.Fi)

- **`BRIDGE 10 USDC FROM POLYGON TO BASE`** — Cross-chain transfer.
- Backend uses **Li.Fi** (quote + execute); supports multiple chains and tokens (e.g. USDC, USDT, ETH, MATIC).
- Mainnet-oriented; testnet tokens may not be supported. Async with SMS notification.

### 4.5 CASHOUT (TXTC → USDC on Arc)

- **`CASHOUT 10 TXTC`** — Convert TXTC to USDC on Arc.
- Flow: burn TXTC from user → swap TXTC→WETH→USDC on Sepolia (Uniswap V3) → **CCTP** `depositForBurn` → Circle attestation → **receiveMessage** on Arc mints USDC to user’s **Circle Developer-Controlled Wallet** (one per phone).
- Arc service holds Circle wallet credentials; supports batch payouts and treasury dashboard APIs.

### 4.6 Other

- **Deposit detection** — Backend (or dedicated monitor) watches user wallets for incoming ETH/ERC20; notifies via SMS.
- **Contacts** — `SAVE name +phone`, `CONTACTS`; stored in DB; used for resolving recipients.
- **Airtime-to-token** — Separate flow (e.g. USSD / airtime gateway) to credit TXTC from mobile airtime (e.g. MTN, Airtel).

---

## 5. Data and Request Flows

- **Inbound SMS** — Twilio → HTTP POST to handler `/sms/incoming` (or similar); handler authenticates (e.g. Twilio signature), parses body, loads user, runs command, returns reply.
- **Outbound SMS** — Handler and backends use Twilio (or same gateway) to send notifications (balance, deposit, swap/bridge/cashout result).
- **Auth** — All commands are scoped by **phone number** (From). No separate login; possession of the phone is the credential.
- **Signing** — For SEND/SWAP/BRIDGE/CASHOUT, the server uses the stored key material for that phone to sign transactions or to interact with services that need user signatures (e.g. Yellow, backend proxy).

---

## 6. Trust Model and Security

### 6.1 Current State: Centralized Server

Today the system is **intentionally centralized** in several ways:

- **SMS gateway** — Twilio (or similar) sees all messages and can deliver or block.
- **SMS Request Handler** — Single point that parses every command and has access to the **user database** (phone → wallet, **encrypted_private_key**).
- **Key material** — User private keys (or their encrypted form) are stored and used on the **same server** (or same trust domain). Encryption at rest and in transit is assumed, but the server can still sign for users.
- **Backend / Yellow / Arc** — Hold backend keys, API keys, and (for Arc) Circle wallet credentials; they execute swaps, bridges, and cashouts on behalf of users.

So: **the server is trusted** for correct execution, availability, and confidentiality of keys. This is the main trust assumption.

### 6.2 Server as TEE or Secure Enclave (Target Model)

To **reduce trust** and improve security without changing the SMS UX, the **SMS Request Handler (and/or the component that stores and uses user keys) can be run inside a Trusted Execution Environment (TEE)** or a **secure enclave**:

- **TEE** (e.g. Intel SGX, AMD SEV, or a confidential-computing cloud) provides attested, encrypted execution: code and data (including private keys) are protected from the host OS and other tenants. Only the attested code can decrypt and sign.
- **Secure enclave** (or HSM) can restrict key export and signing to a well-defined API, so even a compromised app server cannot exfiltrate raw keys.

In that model:

- **User keys** are generated or imported **inside** the TEE/enclave and never leave in plaintext.
- **Signing** happens inside the TEE/enclave; the rest of the stack only receives signed transactions or signatures.
- **Attestation** (e.g. via signed quotes or roots of trust) allows operators and auditors to verify that the correct binary and config are running.

We describe the server as **TEE-capable** or **secure-enclave-ready**: the architecture (single entrypoint, DB with encrypted key material, backend calls) can be refactored so that the **sensitive part** (key storage + signing) runs in a TEE or enclave, while the rest (parsing, routing, notifications) stays in the normal cloud. **Today, the implementation is centralized;** the goal is to make the server **at least secure** (hardened, minimal surface, encrypted storage) and to move toward **TEE or enclave** for the key-handling path so that “server is trusted” becomes “only the attested secure component is trusted.”

### 6.3 Other Security Measures

- Backend/operator wallet keys in env (or secrets manager), not in repo.
- Owner-only contract functions (e.g. `burnFromAny`, `mint`) restricted to backend.
- Phone number as auth; optional PIN for extra protection.
- User funds remain in their own on-chain addresses; settlement is on public chains (Sepolia, Arc, etc.).

---

## 7. Summary

| Aspect | Description |
|--------|-------------|
| **Onboarding** | Single SMS: `JOIN` → wallet created; `JOIN &lt;name&gt;` → ENS; key material stored server-side. |
| **Features** | SEND (Yellow), SWAP (Uniswap V3), BRIDGE (Li.Fi), CASHOUT (CCTP → Arc), redeem, balance, deposit alerts, contacts. |
| **How it works** | SMS → Handler (Rust) → DB + Backend/Yellow/Arc → chains and third-party protocols; server signs for users. |
| **Trust** | Currently centralized (server and backends trusted). Server designed to be **TEE or secure-enclave ready** so key handling can be confined to an attested, secure component. |

This document reflects the architecture and trust model as of the current implementation; specifics (ports, DB type, encryption scheme) may vary by deployment.
