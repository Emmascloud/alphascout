use super::dexscreener::TokenInfo;
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskTier {
    High,
    Elevated,
    Moderate,
    Low,
}

impl RiskTier {
    pub fn label(&self) -> &'static str {
        match self {
            RiskTier::High => "high",
            RiskTier::Elevated => "elevated",
            RiskTier::Moderate => "moderate",
            RiskTier::Low => "low",
        }
    }
}

/// Heuristic risk read built only from data Dexscreener actually exposes:
/// liquidity depth, pair age, volume-to-liquidity ratio, and buy/sell skew.
/// This is NOT a contract audit or holder-distribution check — those need a
/// chain-data provider (e.g. an on-chain RPC + token holder API) that isn't
/// wired up yet. Flags are returned as plain strings so the Aomi LLM can
/// narrate them; we don't pre-format prose here.
pub fn assess_risk(token: &TokenInfo) -> (RiskTier, Vec<String>) {
    let mut flags = Vec::new();
    let liquidity = token.liquidity_usd();
    let age_min = token.age_minutes();
    let vol24 = token.volume.as_ref().and_then(|v| v.h24).unwrap_or(0.0);

    let mut risk_points = 0u8;

    if liquidity < 5_000.0 {
        flags.push("Liquidity under $5k — extremely thin, large slippage risk".to_string());
        risk_points += 3;
    } else if liquidity < 25_000.0 {
        flags.push("Liquidity under $25k — thin, sized for small trades only".to_string());
        risk_points += 2;
    } else if liquidity < 100_000.0 {
        flags.push("Liquidity under $100k — moderate depth".to_string());
        risk_points += 1;
    }

    if let Some(age) = age_min {
        if age < 30 {
            flags.push(format!("Pair is {age} minutes old — pre-discovery stage, unverified"));
            risk_points += 3;
        } else if age < 360 {
            flags.push(format!("Pair is {}h old — still in early price-discovery window", age / 60));
            risk_points += 1;
        }
    } else {
        flags.push("Pair creation time unavailable — cannot confirm launch recency".to_string());
        risk_points += 1;
    }

    if liquidity > 0.0 {
        let vol_to_liq = vol24 / liquidity;
        if vol_to_liq > 20.0 {
            flags.push(format!(
                "24h volume is {vol_to_liq:.1}x liquidity — abnormal churn, possible wash trading"
            ));
            risk_points += 2;
        }
    }

    if let Some(txns) = token.txns.as_ref().and_then(|t| t.h24.as_ref()) {
        let buys = txns.buys.unwrap_or(0) as f64;
        let sells = txns.sells.unwrap_or(0) as f64;
        let total = buys + sells;
        if total > 10.0 {
            let sell_ratio = sells / total;
            if sell_ratio > 0.75 {
                flags.push(format!(
                    "Sell-heavy: {:.0}% of 24h txns are sells — possible distribution/dump pattern",
                    sell_ratio * 100.0
                ));
                risk_points += 2;
            }
        }
    }

    if let Some(change) = token.price_change.as_ref().and_then(|p| p.h1) {
        if change.abs() > 50.0 {
            flags.push(format!("1h price move of {change:+.0}% — extreme volatility"));
            risk_points += 1;
        }
    }

    if flags.is_empty() {
        flags.push("No major heuristic red flags from available market data".to_string());
    }

    let tier = match risk_points {
        0..=1 => RiskTier::Low,
        2..=3 => RiskTier::Moderate,
        4..=6 => RiskTier::Elevated,
        _ => RiskTier::High,
    };

    (tier, flags)
}

/// Structured per-token summary. Tools return this as JSON — formatting into
/// prose/emoji is the Aomi LLM's job, not ours.
pub fn token_summary(token: &TokenInfo) -> Value {
    let (tier, flags) = assess_risk(token);

    json!({
        "name": token.base_token.name,
        "symbol": token.base_token.symbol,
        "chain": token.chain_id,
        "dex": token.dex_id,
        "pair_address": token.pair_address,
        "token_address": token.base_token.address,
        "price_usd": token.price_usd,
        "liquidity_usd": token.liquidity_usd(),
        "fdv": token.fdv,
        "market_cap": token.market_cap,
        "volume_24h": token.volume.as_ref().and_then(|v| v.h24),
        "price_change_1h": token.price_change.as_ref().and_then(|p| p.h1),
        "price_change_24h": token.price_change.as_ref().and_then(|p| p.h24),
        "pair_age_minutes": token.age_minutes(),
        "dexscreener_url": token.url,
        "risk_tier": tier.label(),
        "risk_flags": flags,
    })
}

pub fn summarize_many(tokens: &[TokenInfo]) -> Value {
    json!({
        "count": tokens.len(),
        "tokens": tokens.iter().map(token_summary).collect::<Vec<_>>(),
    })
}
