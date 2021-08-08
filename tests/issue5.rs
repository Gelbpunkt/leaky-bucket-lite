use leaky_bucket_lite::LeakyBucket;

use std::time::{Duration, Instant};

#[tokio::test]
async fn test_issue5_a() {
    let rate_limiter = LeakyBucket::builder()
        .refill_amount(1.0)
        .refill_interval(Duration::from_millis(100))
        .build();

    let begin = Instant::now();

    for _ in 0..10 {
        rate_limiter.acquire_one().await.expect("No reason to fail");
    }

    let elapsed = Instant::now().duration_since(begin);
    println!("Elapsed: {:?}", elapsed);
    assert!((elapsed.as_secs_f64() - 1.).abs() < 0.1);
}

#[tokio::test]
async fn test_issue5_b() {
    let rate_limiter = LeakyBucket::builder()
        .refill_amount(1.0)
        .refill_interval(Duration::from_secs(2))
        .build();

    let begin = Instant::now();

    for _ in 0..2 {
        rate_limiter.acquire_one().await.expect("No reason to fail");
    }

    let elapsed = Instant::now().duration_since(begin);
    println!("Elapsed: {:?}", elapsed);
    // once per 2 seconds => 4 seconds for 2 permits
    assert!((elapsed.as_secs_f64() - 4.).abs() < 0.1);
}
