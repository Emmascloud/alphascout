# AlphaScout

**Live Web3 token launch intelligence — built on Aomi.**

AlphaScout scans Dexscreener for newly-created token pairs, scores each one against heuristic risk signals, and delivers structured research reports and investment pitches. It runs as an Aomi community app, meaning the Aomi LLM drives the conversation while AlphaScout's tools supply the live on-chain market data.

> Not financial advice. AlphaScout is a research starting point, not a trading signal.

---

## What it does

- **Scouts new launches** — finds token pairs created in the last 30 minutes to 24 hours across any chain (Base, Ethereum, Solana, etc.)
- **Risk-scores each token** — liquidity depth, pair age, volume/liquidity ratio, buy/sell skew, short-term volatility
- **Prepares a pitch** — structured investment-thesis-style writeup with a manual diligence checklist for what AlphaScout cannot verify automatically (contract ownership, holder concentration, LP lock)
- **Surfaces high-conviction picks** — filters recent launches by liquidity depth to show the strongest signals first

## Live demo

**Web UI:** https://emmascloud.github.io/alphascout/

Open it, hit SCAN, and see live token launches with risk tiers and full pitch panels — no setup required.

## Target users

Crypto researchers and degens who want to spot new token launches early, get a quick risk read, and know exactly what to verify before going further. Not for passive investors — for active researchers who do their own diligence.

---

## Aomi integration

AlphaScout is built as an Aomi SDK plugin (`cdylib` + `rlib`). Four tools are registered:

| Tool | What it does |
|------|-------------|
| `scout_new_tokens` | Scan for newly-created pairs, age-filtered, risk-scored |
| `analyze_token` | Deep-dive report on one specific pair |
| `pitch_token` | Investment-thesis pitch with diligence checklist |
| `high_conviction_picks` | Filtered scan — strongest recent launches by liquidity |

The Aomi LLM routes user questions to the right tool and formats the structured JSON into natural-language reports. AlphaScout supplies the live data; Aomi supplies the conversational interface.

---

## Stack

- **Rust** — Aomi SDK plugin (`aomi-sdk = "0.1.20"`), Axum backend
- **Dexscreener API** — live token pair data, free public endpoint
- **Node.js** — Telegram bot (optional interface)
- **HTML/JS** — standalone web UI, no build step, deployable anywhere

## Run locally

```bash
# Build everything
cargo build

# Start the backend (port 8000)
cargo run -p alphascout-backend

# Test the API
curl -X POST http://localhost:8000/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "scout new tokens"}'
```

## Risk model — what it checks and what it doesn't

AlphaScout's heuristic risk tier is built from market data only:

✅ Liquidity depth (USD)  
✅ Pair age (minutes since creation)  
✅ 24h volume vs liquidity ratio (wash-trading signal)  
✅ Buy/sell transaction skew  
✅ Short-term price volatility (1h move)  

❌ Contract ownership / renouncement — requires on-chain RPC  
❌ Token holder concentration — requires holder API  
❌ LP lock status — requires lock contract verification  

The `pitch_token` tool always returns a diligence checklist of the unverified items. AlphaScout is explicit about what it doesn't know.

---

Built for the [Aomi Early Forge](https://aomi.dev) program.
