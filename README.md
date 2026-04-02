# 🌉 Stellar Money Bridge

> Bridging Stablecoins to Real-World Money Systems — with Built-in Recurring Payments

Stellar Money Bridge is a Soroban-powered infrastructure that enables seamless conversion between Stellar-based stablecoins (USDC) and local fiat systems such as mobile money and bank transfers. It now includes the **Soroban Subscription Engine** — a trustless, on-chain recurring payment protocol for Web3 billing.

---

## Why This Exists

Recurring payments power most of the modern digital economy — SaaS, memberships, payroll, API billing. But in Web3:

- Payments are mostly manual
- There is no native recurring billing standard
- Users lack control and transparency
- Developers must build custom solutions from scratch

Stellar Money Bridge solves the fiat conversion layer. The Subscription Engine solves the recurring billing layer. Together they form a complete payment infrastructure on Stellar.

---

## Architecture

```
Subscriber / Merchant
        │
        ▼
 Subscription Engine  ◄──── Off-chain Relayer (cron / event-driven)
        │
        ▼
   Escrow Contract  ──────► Liquidity Providers / Anchors
        │
        ▼
   Router Contract  ──────► Mobile Money / Bank Rails
```

---

## Contracts

| Contract | Description |
|---|---|
| `contracts/subscription` | Recurring billing engine — plans, subscriptions, billing cycles, grace periods |
| `contracts/escrow` | Locks USDC until off-chain settlement is confirmed, then releases or refunds |
| `contracts/router` | Selects the cheapest active liquidity provider for a given fiat rail |

---

## Subscription Engine

The core of the recurring payment protocol. Fully on-chain, no custodians.

### Concepts

| Term | Description |
|---|---|
| Plan | Created by a merchant. Defines token, amount, and billing cycle |
| Subscription | A subscriber's enrollment into a plan |
| Relayer | A permissioned off-chain service that triggers `execute_billing` when a cycle is due |
| Grace Period | 24-hour window after a failed charge before the subscription is cancelled |

### Billing Cycles

| Cycle | Interval |
|---|---|
| `Daily` | 86,400 seconds |
| `Weekly` | 604,800 seconds |
| `Monthly` | 2,592,000 seconds (~30 days) |

### Contract Interface

```rust
// Deploy
initialize(admin: Address, relayer: Address)

// Merchant
create_plan(merchant, token, amount, cycle) -> plan_id
deactivate_plan(merchant, plan_id)

// Subscriber
subscribe(subscriber, plan_id)   // charges first payment immediately
pause(subscriber, plan_id)
resume(subscriber, plan_id)
cancel(subscriber, plan_id)

// Relayer
execute_billing(subscriber, plan_id)

// Read
get_plan(plan_id) -> Plan
get_subscription(subscriber, plan_id) -> Subscription
plan_count() -> u32
```

### Subscription Lifecycle

```
subscribe()
    │
    ▼
 [Active] ──── pause() ────► [Paused] ──── resume() ────► [Active]
    │                                                          │
    │◄─────────────────── execute_billing() ──────────────────┘
    │
    ├── insufficient balance ──► [GracePeriod] ── retry within 24h ──► [Active]
    │                                    │
    │                                    └── grace expires ──► [Cancelled]
    │
    └── cancel() ──► [Cancelled]
```

### Events Emitted

| Event | Trigger |
|---|---|
| `plan_new` | New plan created |
| `subscribed` | Subscriber enrolled |
| `billed` | Successful charge |
| `grace` | Charge failed, grace period started |
| `lapsed` | Grace period expired, subscription cancelled |
| `cancelled` | Subscriber cancelled manually |

---

## Quick Start

### Prerequisites

- Rust + `wasm32-unknown-unknown` target
- [`soroban-cli`](https://soroban.stellar.org/docs/getting-started/setup)
- Stellar testnet account + funded wallet
- Node.js 18+ (for backend/frontend)

```bash
rustup target add wasm32-unknown-unknown
cargo install --locked soroban-cli
```

### Build All Contracts

```bash
cargo build --workspace --target wasm32-unknown-unknown --release
```

Or individually:

```bash
cd contracts/subscription && cargo build --target wasm32-unknown-unknown --release
cd contracts/escrow       && cargo build --target wasm32-unknown-unknown --release
cd contracts/router       && cargo build --target wasm32-unknown-unknown --release
```

### Deploy to Testnet

```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/soroban_subscription_engine.wasm \
  --source <YOUR_SECRET_KEY> \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015"
```

### Initialize the Subscription Contract

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source <YOUR_SECRET_KEY> \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015" \
  -- initialize \
  --admin <ADMIN_ADDRESS> \
  --relayer <RELAYER_ADDRESS>
```

### Backend

```bash
cd backend && npm install && npm run dev
```

### Frontend

```bash
cd frontend && npm install && npm run dev
```

---

## Relayer

The relayer is an off-chain service (cron job or event-driven worker) that:

1. Indexes all active subscriptions
2. Checks `next_billing <= now` for each
3. Calls `execute_billing(subscriber, plan_id)` on-chain
4. Handles retry logic within the grace period window

A reference relayer implementation will live in `relayer/`.

---

## Use Cases

- SaaS subscription billing
- Membership platforms
- Payroll and recurring payouts
- API usage billing
- Content creator monetization
- DAO treasury recurring payments

---

## Project Structure

```
.
├── Cargo.toml                        # Workspace root
├── contracts/
│   ├── subscription/                 # Soroban Subscription Engine
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── escrow/                       # USDC escrow contract
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   └── router/                       # Liquidity provider router
│       ├── Cargo.toml
│       └── src/lib.rs
├── backend/                          # REST API (coming soon)
├── frontend/                         # Merchant + subscriber dashboard (coming soon)
├── relayer/                          # Off-chain billing relayer (coming soon)
└── sdk/                              # Developer SDK (coming soon)
```

---

## Roadmap

| Phase | Scope |
|---|---|
| Phase 1 | Core smart contracts — subscription engine, escrow, router |
| Phase 2 | Off-chain relayer + indexer + REST API |
| Phase 3 | Merchant dashboard (React/Next.js) + subscriber portal |
| Phase 4 | Multi-token support, token streaming, split payments |
| Phase 5 | Cross-chain support + SDK + developer docs |

---

## Contributing

PRs and issues welcome. Please open an issue before submitting large changes.

---

## License

MIT
