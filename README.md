# 🌉 Stellar Money Bridge

> Bridging Stablecoins to Real-World Money Systems

Stellar Money Bridge is a Soroban-powered infrastructure that enables seamless conversion between Stellar-based stablecoins (USDC) and local fiat systems such as mobile money and bank transfers.

## Architecture

```
User → Frontend / API → Soroban Smart Contract → Liquidity Providers / Anchors → Mobile Money / Bank
```

## Packages

| Package | Description |
|---|---|
| `contracts/` | Soroban smart contracts (routing, escrow, settlement) |
| `backend/` | REST API layer (orchestration + compliance) |
| `frontend/` | Transaction dashboard |
| `sdk/` | Developer SDK for integrations |

## Quick Start

### Prerequisites
- Rust + `soroban-cli`
- Node.js 18+
- Stellar testnet account

### Contracts
```bash
cd contracts/router && cargo build --target wasm32-unknown-unknown --release
cd contracts/escrow && cargo build --target wasm32-unknown-unknown --release
```

### Backend
```bash
cd backend && npm install && npm run dev
```

### Frontend
```bash
cd frontend && npm install && npm run dev
```

## MVP Scope
- Single corridor: US → Nigeria
- USDC ↔ NGN conversion
- One mobile money integration (MTN)
- Basic REST API + simple UI

## Roadmap
- **Phase 1**: Core smart contracts + basic USDC → fiat conversion
- **Phase 2**: Mobile money integrations + Developer API + dashboard
- **Phase 3**: Multi-country expansion + advanced routing
- **Phase 4**: Fintech partnerships + compliance + scaling
