use leaky_bucket_lite::Builder;

use std::time::{Duration, Instant};

const RESET_DURATION_MILLISECONDS: u64 = 60_000;
const COMMANDS_ALLOWED_WITHOUT_HEARTBEATS: u32 = 118;

#[tokio::test]
async fn test_discord() {
    let leaky = Builder::new()
        .tokens(COMMANDS_ALLOWED_WITHOUT_HEARTBEATS)
        .max(COMMANDS_ALLOWED_WITHOUT_HEARTBEATS)
        .refill_amount(COMMANDS_ALLOWED_WITHOUT_HEARTBEATS)
        .refill_interval(Duration::from_millis(RESET_DURATION_MILLISECONDS))
        .build();

    let start = Instant::now();

    // Now, send 250 commands
    for _ in 0..250 {
        leaky.acquire_one().await;
    }

    let end = Instant::now();
    let elapsed = end - start;

    assert!(
        elapsed.as_millis() >= (RESET_DURATION_MILLISECONDS * 2).into()
            && elapsed.as_millis() < (RESET_DURATION_MILLISECONDS * 2 + 50).into()
    );
}
