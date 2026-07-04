use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct TokenInfo {
    #[serde(rename = "chainId")]
    pub chain_id: String,

    #[serde(rename = "dexId")]
    pub dex_id: String,

    #[serde(rename = "pairAddress")]
    pub pair_address: String,

    pub url: Option<String>,

    #[serde(rename = "priceUsd")]
    pub price_usd: Option<String>,

    #[serde(rename = "baseToken")]
    pub base_token: BaseToken,

    pub liquidity: Option<Liquidity>,

    #[serde(rename = "fdv")]
    pub fdv: Option<f64>,

    #[serde(rename = "marketCap")]
    pub market_cap: Option<f64>,

    #[serde(rename = "volume")]
    pub volume: Option<Volume>,

    #[serde(rename = "priceChange")]
    pub price_change: Option<PriceChange>,

    #[serde(rename = "txns")]
    pub txns: Option<Txns>,

    /// Unix ms timestamp of pair creation. This is the key field for
    /// "new launch" detection — Dexscreener's /search endpoint does not
    /// sort by this, so we sort and filter on it client-side.
    #[serde(rename = "pairCreatedAt")]
    pub pair_created_at: Option<i64>,
}

impl TokenInfo {
    /// Age of the pair in minutes, if known.
    pub fn age_minutes(&self) -> Option<i64> {
        let created = self.pair_created_at?;
        let now = chrono::Utc::now().timestamp_millis();
        Some((now - created) / 60_000)
    }

    pub fn liquidity_usd(&self) -> f64 {
        self.liquidity.as_ref().and_then(|l| l.usd).unwrap_or(0.0)
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct BaseToken {
    pub name: String,
    pub symbol: String,
    pub address: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Liquidity {
    pub usd: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Volume {
    pub h24: Option<f64>,
    pub h6: Option<f64>,
    pub h1: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PriceChange {
    pub h24: Option<f64>,
    pub h6: Option<f64>,
    pub h1: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TxnCounts {
    pub buys: Option<u32>,
    pub sells: Option<u32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Txns {
    pub h24: Option<TxnCounts>,
    pub h1: Option<TxnCounts>,
}

#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    pub pairs: Option<Vec<TokenInfo>>,
}

/// Search Dexscreener pairs by free-text query (chain name, token symbol, etc).
/// Dexscreener does not expose a true "new pairs" firehose on the free search
/// endpoint, so callers should sort the result by `pair_created_at` and filter
/// by `age_minutes()` to approximate a live new-launch feed.
pub async fn search_pairs(query: &str) -> Result<Vec<TokenInfo>, String> {
    let url = format!(
        "https://api.dexscreener.com/latest/dex/search?q={}",
        urlencode(query)
    );

    let response = reqwest::get(&url)
        .await
        .map_err(|e| format!("dexscreener request failed: {e}"))?
        .json::<SearchResponse>()
        .await
        .map_err(|e| format!("dexscreener response parse failed: {e}"))?;

    Ok(response.pairs.unwrap_or_default())
}

/// Convenience wrapper kept for backwards compatibility: latest Base pairs,
/// capped at 10, unsorted (raw Dexscreener relevance order).
pub async fn latest_tokens() -> Result<Vec<TokenInfo>, String> {
    let pairs = search_pairs("base").await?;
    Ok(pairs.into_iter().take(10).collect())
}

/// New-launch scan: search a chain/query, sort newest-first, and keep only
/// pairs younger than `max_age_minutes`. This is the function the
/// `scout_new_tokens` Aomi tool calls.
pub async fn scout_new_pairs(
    query: &str,
    max_age_minutes: i64,
    limit: usize,
) -> Result<Vec<TokenInfo>, String> {
    let mut pairs = search_pairs(query).await?;

    pairs.sort_by(|a, b| {
        b.pair_created_at
            .unwrap_or(0)
            .cmp(&a.pair_created_at.unwrap_or(0))
    });

    let filtered: Vec<TokenInfo> = pairs
        .into_iter()
        .filter(|p| match p.age_minutes() {
            Some(age) => age >= 0 && age <= max_age_minutes,
            None => false,
        })
        .take(limit)
        .collect();

    Ok(filtered)
}

/// Fetch a single pair by its pair address, for deep-dive analysis.
pub async fn get_pair(chain_id: &str, pair_address: &str) -> Result<TokenInfo, String> {
    let url = format!(
        "https://api.dexscreener.com/latest/dex/pairs/{}/{}",
        urlencode(chain_id),
        urlencode(pair_address)
    );

    let response = reqwest::get(&url)
        .await
        .map_err(|e| format!("dexscreener request failed: {e}"))?
        .json::<SearchResponse>()
        .await
        .map_err(|e| format!("dexscreener response parse failed: {e}"))?;

    response
        .pairs
        .and_then(|mut p| if p.is_empty() { None } else { Some(p.remove(0)) })
        .ok_or_else(|| format!("no pair found for {chain_id}/{pair_address}"))
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        let c = *b as char;
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
            out.push(c);
        } else {
            use std::fmt::Write;
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}
