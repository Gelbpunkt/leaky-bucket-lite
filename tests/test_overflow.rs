use leaky_bucket_lite::LeakyBucket;
use std::time::{Duration, Instant};

#[tokio::test]
async fn test_overflow() {
    let rate_limiter = LeakyBucket::builder()
        .max(5)
        .tokens(5)
        .refill_amount(1)
        .refill_interval(Duration::from_millis(100))
        .build();

    let begin = Instant::now();

    for _ in 0..10 {
        rate_limiter.acquire_one().await.expect("No reason to fail");
    }

    let elapsed = Instant::now().duration_since(begin);
    println!("Elapsed: {:?}", elapsed);
    assert!(elapsed.as_millis() >= 500 && elapsed.as_millis() <= 550);
}

#[tokio::test]
async fn test_overflow_2() {
    let rate_limiter = LeakyBucket::builder()
        .max(5)
        .tokens(5)
        .refill_amount(1)
        .refill_interval(Duration::from_millis(100))
        .build();

    let begin = Instant::now();

    rate_limiter.acquire(10).await.expect("No reason to fail");

    let elapsed = Instant::now().duration_since(begin);
    println!("Elapsed: {:?}", elapsed);
    assert!(elapsed.as_millis() >= 500 && elapsed.as_millis() <= 550);
}
