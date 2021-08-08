use leaky_bucket_lite::LeakyBucket;

use std::time::{Duration, Instant};

#[tokio::test]
async fn test_ao_issue() {
    let rate_limiter = LeakyBucket::builder()
        .refill_amount(1.0)
        .refill_interval(Duration::from_secs(2))
        .max(5.0)
        .tokens(5.0)
        .build();

    let begin = Instant::now();

    tokio::time::sleep(Duration::from_secs(10)).await;

    for _ in 0..10 {
        rate_limiter.acquire_one().await.expect("No reason to fail");
    }

    let elapsed = Instant::now().duration_since(begin);
    println!("Elapsed: {:?}", elapsed);
    assert!(elapsed.as_millis() >= 20000 && elapsed.as_millis() <= 20050);
}

#[tokio::test]
async fn test_ao_issue_2() {
    let rate_limiter = LeakyBucket::builder()
        .refill_amount(1.0)
        .refill_interval(Duration::from_secs(2))
        .max(5.0)
        .tokens(5.0)
        .build();

    tokio::time::sleep(Duration::from_secs(11)).await;

    rate_limiter.acquire_one().await.expect("No reason to fail");
    let first_acquire = Instant::now();

    for _ in 0..4 {
        rate_limiter.acquire_one().await.expect("No reason to fail");
    }
    rate_limiter.acquire_one().await.expect("No reason to fail");
    let elapsed = Instant::now().duration_since(first_acquire);
    println!("Elapsed: {:?}", elapsed);
    assert!((elapsed.as_secs_f64() - 1.0).abs() < 0.1);
}
