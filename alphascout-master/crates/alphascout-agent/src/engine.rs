//! Plain-text entry point used by the HTTP backend (and from there, the
//! Telegram bot / future web UI). This is a thin wrapper around the same
//! Dexscreener + risk-analysis functions the Aomi tools call in `tool.rs` —
//! kept here so non-Aomi clients (raw REST, Telegram) get useful output
//! without needing an LLM in the loop to format structured JSON.
//!
//! For the full conversational experience (free-form questions, multi-turn
//! reasoning about a token), route through the Aomi app itself — this path
//! is the lightweight command fallback.

use crate::market::analyzer::{summarize_many, token_summary};
use crate::market::dexscreener::scout_new_pairs;

pub async fn process_message(message: String) -> String {
    let lower = message.to_lowercase();

    if lower.contains("new") || lower.contains("scout") || lower.contains("scan") {
        return scan_reply("base", 60, 10).await;
    }

    if lower.contains("conviction") || lower.contains("top") || lower.contains("best") {
        return scan_reply("base", 360, 5).await;
    }

    "AlphaScout here. Try:\n\
     • \"scout new tokens\" — recent Base launches\n\
     • \"high conviction\" — top recent launches by liquidity\n\
     For a deep dive on one token, use the AlphaScout Aomi agent directly, which can \
     take a specific pair address."
        .to_string()
}

async fn scan_reply(query: &str, max_age_minutes: i64, limit: usize) -> String {
    match scout_new_pairs(query, max_age_minutes, limit).await {
        Ok(tokens) if tokens.is_empty() => format!(
            "No pairs found younger than {max_age_minutes} minutes for '{query}'. \
             Try again shortly — new pairs surface continuously."
        ),
        Ok(tokens) => {
            let summary = summarize_many(&tokens);
            format_for_text(&summary)
        }
        Err(e) => format!("Scan failed: {e}"),
    }
}

fn format_for_text(summary: &serde_json::Value) -> String {
    let mut out = String::new();
    if let Some(tokens) = summary.get("tokens").and_then(|t| t.as_array()) {
        for t in tokens {
            let name = t.get("name").and_then(|v| v.as_str()).unwrap_or("?");
            let symbol = t.get("symbol").and_then(|v| v.as_str()).unwrap_or("?");
            let liq = t.get("liquidity_usd").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let age = t.get("pair_age_minutes").and_then(|v| v.as_i64());
            let tier = t.get("risk_tier").and_then(|v| v.as_str()).unwrap_or("?");
            let pair = t.get("pair_address").and_then(|v| v.as_str()).unwrap_or("?");

            out.push_str(&format!(
                "🪙 {name} ({symbol})\n   Liquidity: ${liq:.0} | Age: {} | Risk: {tier}\n   Pair: {pair}\n\n",
                age.map(|a| format!("{a}m")).unwrap_or_else(|| "?".to_string())
            ));
        }
    }
    if out.is_empty() {
        "No results.".to_string()
    } else {
        out
    }
}

pub fn token_report(summary: &serde_json::Value) -> String {
    let name = summary.get("name").and_then(|v| v.as_str()).unwrap_or("?");
    let symbol = summary.get("symbol").and_then(|v| v.as_str()).unwrap_or("?");
    let tier = summary.get("risk_tier").and_then(|v| v.as_str()).unwrap_or("?");
    let flags = summary
        .get("risk_flags")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|f| f.as_str())
                .map(|f| format!("  - {f}"))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();

    format!("{name} ({symbol})\nRisk: {tier}\n{flags}")
}

// Re-export for callers that want a single-token summary directly.
pub use crate::market::analyzer::token_summary as analyze_token_summary;
