use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Spin up a fresh tokio runtime for use inside a synchronous `DynAomiTool::run`.
/// The Aomi FFI dispatches one tool call at a time (synchronous), so each tool
/// creates its own short-lived runtime to drive async HTTP calls underneath.
pub(crate) fn rt() -> Result<tokio::runtime::Runtime, String> {
    tokio::runtime::Runtime::new().map_err(|e| format!("[alphascout] tokio runtime: {e}"))
}

// Dexscreener's public API is rate-limited; keep calls spaced out so we don't
// get 429s mid-scan. One outstanding request, minimum 300ms between calls.
static LAST_CALL: Mutex<Option<Instant>> = Mutex::new(None);
const MIN_INTERVAL: Duration = Duration::from_millis(300);

/// Enforce a minimum gap between Dexscreener requests.
/// Called inside every tool's `block_on` block before the HTTP call.
pub(crate) async fn rate_gate() -> Result<(), String> {
    let sleep_for = {
        let mut guard = LAST_CALL
            .lock()
            .map_err(|e| format!("[alphascout] rate-limiter lock poisoned: {e}"))?;
        let now = Instant::now();
        let sleep_for = guard
            .map(|last| {
                let elapsed = now.duration_since(last);
                if elapsed < MIN_INTERVAL {
                    MIN_INTERVAL - elapsed
                } else {
                    Duration::ZERO
                }
            })
            .unwrap_or(Duration::ZERO);
        *guard = Some(now + sleep_for);
        sleep_for
    };
    if sleep_for > Duration::ZERO {
        tokio::time::sleep(sleep_for).await;
    }
    Ok(())
}
