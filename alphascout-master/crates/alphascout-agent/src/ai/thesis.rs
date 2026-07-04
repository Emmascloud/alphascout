use crate::market::analyzer::assess_risk;
use crate::market::dexscreener::TokenInfo;
use serde_json::{json, Value};

/// Structured pitch data for one token: the raw facts plus a risk read.
/// The Aomi LLM turns this into the actual pitch copy — we just supply
/// grounded numbers and flags so it isn't inventing them.
pub fn build_thesis(token: &TokenInfo) -> Value {
    let (tier, flags) = assess_risk(token);
    let liquidity = token.liquidity_usd();
    let vol24 = token.volume.as_ref().and_then(|v| v.h24);

    json!({
        "token": format!("{} ({})", token.base_token.name, token.base_token.symbol),
        "chain": token.chain_id,
        "dex": token.dex_id,
        "pair_address": token.pair_address,
        "price_usd": token.price_usd,
        "liquidity_usd": liquidity,
        "volume_24h": vol24,
        "pair_age_minutes": token.age_minutes(),
        "risk_tier": tier.label(),
        "risk_flags": flags,
        "diligence_checklist": [
            "Verify contract is renounced or has a reasonable timelock",
            "Check token holder concentration (top 10 wallets)",
            "Confirm LP tokens are locked or burned",
            "Review team/socials for credibility signals",
            "Check for prior rug history on the deployer address"
        ],
        "diligence_note": "AlphaScout's risk read is built from market data only (liquidity, age, volume, txn skew). Contract, holder, and LP-lock checks above are NOT yet automated — verify manually before acting.",
    })
}
