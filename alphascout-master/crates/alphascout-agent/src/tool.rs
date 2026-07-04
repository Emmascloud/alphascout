use aomi_sdk::{DynAomiTool, DynToolCallCtx};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::client::rt;
use crate::market::analyzer::{summarize_many, token_summary};
use crate::market::dexscreener::{get_pair, scout_new_pairs};
use crate::ai::thesis::build_thesis;

#[derive(Clone, Default)]
pub struct AlphaScoutApp;

// ============================================================================
// Tool 1: ScoutNewTokens
// ============================================================================

pub struct ScoutNewTokens;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ScoutNewTokensArgs {
    /// Chain or free-text search term to scope the scan, e.g. "base",
    /// "ethereum", or a token symbol fragment. Defaults to "base".
    #[serde(default)]
    pub query: Option<String>,
    /// Only include pairs created within this many minutes. Default 60,
    /// max 1440 (24h).
    #[serde(default)]
    pub max_age_minutes: Option<i64>,
    /// Max number of results to return. Default 10, max 25.
    #[serde(default)]
    pub limit: Option<u32>,
}

impl DynAomiTool for ScoutNewTokens {
    type App = AlphaScoutApp;
    type Args = ScoutNewTokensArgs;

    const NAME: &'static str = "scout_new_tokens";
    const DESCRIPTION: &'static str = "Scan Dexscreener for newly-created token pairs. \
        Use when the user asks to discover new token launches, find what's fresh on a \
        chain, or scout for emerging opportunities. Returns structured data per token: \
        price, liquidity, volume, pair age, and an AlphaScout heuristic risk tier with \
        flags. Summarize the list and call out the most notable entries — do not just \
        repeat raw numbers. Public — no API key needed.";

    fn run(_app: &AlphaScoutApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        let query = args.query.unwrap_or_else(|| "base".to_string());
        let max_age = args.max_age_minutes.unwrap_or(60).clamp(1, 1440);
        let limit = args.limit.unwrap_or(10).min(25) as usize;

        let rt = rt()?;
        rt.block_on(async move {
            let tokens = scout_new_pairs(&query, max_age, limit).await?;
            if tokens.is_empty() {
                return Ok(json!({
                    "count": 0,
                    "tokens": [],
                    "note": format!(
                        "No pairs found younger than {max_age} minutes for query '{query}'. \
                         Try a wider max_age_minutes or a different query."
                    ),
                }));
            }
            Ok(summarize_many(&tokens))
        })
    }
}

// ============================================================================
// Tool 2: AnalyzeToken
// ============================================================================

pub struct AnalyzeToken;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnalyzeTokenArgs {
    /// Dexscreener chain ID, e.g. "base", "ethereum", "solana".
    pub chain_id: String,
    /// The pair (liquidity pool) address to analyze — not the token contract
    /// address. Get this from `scout_new_tokens` results (`pair_address` field).
    pub pair_address: String,
}

impl DynAomiTool for AnalyzeToken {
    type App = AlphaScoutApp;
    type Args = AnalyzeTokenArgs;

    const NAME: &'static str = "analyze_token";
    const DESCRIPTION: &'static str = "Deep-dive on a single token pair by chain and pair \
        address. Use when the user wants a full risk/opportunity report on one specific \
        token, typically after scout_new_tokens surfaced it. Returns price, liquidity, \
        volume, price-change windows, and a detailed risk-flag breakdown. Summarize this \
        as a clear research report in plain language, calling out every risk flag. \
        Public — no API key needed.";

    fn run(_app: &AlphaScoutApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        let rt = rt()?;
        rt.block_on(async move {
            let token = get_pair(&args.chain_id, &args.pair_address).await?;
            Ok(token_summary(&token))
        })
    }
}

// ============================================================================
// Tool 3: PitchToken
// ============================================================================

pub struct PitchToken;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PitchTokenArgs {
    /// Dexscreener chain ID, e.g. "base", "ethereum", "solana".
    pub chain_id: String,
    /// The pair (liquidity pool) address to build a pitch for.
    pub pair_address: String,
}

impl DynAomiTool for PitchToken {
    type App = AlphaScoutApp;
    type Args = PitchTokenArgs;

    const NAME: &'static str = "pitch_token";
    const DESCRIPTION: &'static str = "Build an investment-thesis-style pitch for one token, \
        including a due-diligence checklist of items AlphaScout has NOT verified \
        on-chain (contract ownership, holder concentration, LP lock). Use when the user \
        wants a 'should I look into this' style pitch rather than raw data. Always present \
        the diligence_note and diligence_checklist alongside the pitch — never frame this \
        as financial advice or a guarantee, this is a research starting point only. \
        Public — no API key needed.";

    fn run(_app: &AlphaScoutApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        let rt = rt()?;
        rt.block_on(async move {
            let token = get_pair(&args.chain_id, &args.pair_address).await?;
            Ok(build_thesis(&token))
        })
    }
}

// ============================================================================
// Tool 4: HighConvictionPicks
// ============================================================================

pub struct HighConvictionPicks;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct HighConvictionPicksArgs {
    /// Chain or free-text search term, e.g. "base". Defaults to "base".
    #[serde(default)]
    pub query: Option<String>,
    /// Only include pairs created within this many minutes. Default 360 (6h).
    #[serde(default)]
    pub max_age_minutes: Option<i64>,
}

impl DynAomiTool for HighConvictionPicks {
    type App = AlphaScoutApp;
    type Args = HighConvictionPicksArgs;

    const NAME: &'static str = "high_conviction_picks";
    const DESCRIPTION: &'static str = "Scan recent token launches and filter down to the \
        ones with the strongest combination of liquidity depth and low heuristic risk \
        flags. Use when the user asks for 'best', 'top picks', 'high conviction', or \
        'alpha' rather than a full unfiltered scan. Still returns risk flags for every \
        result — 'high conviction' here means relatively stronger market signals, not a \
        guarantee. Public — no API key needed.";

    fn run(_app: &AlphaScoutApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        let query = args.query.unwrap_or_else(|| "base".to_string());
        let max_age = args.max_age_minutes.unwrap_or(360).clamp(1, 1440);

        let rt = rt()?;
        rt.block_on(async move {
            let tokens = scout_new_pairs(&query, max_age, 25).await?;

            let mut scored: Vec<_> = tokens
                .iter()
                .filter(|t| t.liquidity_usd() > 50_000.0)
                .collect();

            scored.sort_by(|a, b| {
                b.liquidity_usd()
                    .partial_cmp(&a.liquidity_usd())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let top: Vec<_> = scored.into_iter().take(5).collect();

            if top.is_empty() {
                return Ok(json!({
                    "count": 0,
                    "tokens": [],
                    "note": "No recent launches cleared the $50k liquidity bar in this window. \
                             Try a longer max_age_minutes.",
                }));
            }

            Ok(json!({
                "count": top.len(),
                "tokens": top.iter().map(|t| token_summary(t)).collect::<Vec<_>>(),
            }))
        })
    }
}
