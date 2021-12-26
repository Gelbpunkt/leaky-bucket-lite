use leaky_bucket_lite::{sync_threadsafe::LeakyBucket as SyncLeakyBucket, LeakyBucket};

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
        rate_limiter.acquire_one().await;
    }

    let elapsed = Instant::now().duration_since(begin);
    println!("Elapsed: {:?}", elapsed);
    assert!(elapsed.as_millis() >= 500 && elapsed.as_millis() <= 550);
}

#[tokio::test]
#[should_panic]
async fn test_overflow_2() {
    let rate_limiter = LeakyBucket::builder()
        .max(5)
        .tokens(5)
        .refill_amount(1)
        .refill_interval(Duration::from_millis(100))
        .build();

    let begin = Instant::now();

    rate_limiter.acquire(10).await;

    let elapsed = Instant::now().duration_since(begin);
    println!("Elapsed: {:?}", elapsed);
    assert!(elapsed.as_millis() >= 500 && elapsed.as_millis() <= 550);
}

#[test]
fn test_overflow_sync_threadsafe() {
    let rate_limiter = SyncLeakyBucket::builder()
        .max(5)
        .tokens(5)
        .refill_amount(1)
        .refill_interval(Duration::from_millis(100))
        .build();

    let begin = Instant::now();

    for _ in 0..10 {
        rate_limiter.acquire_one();
    }

    let elapsed = Instant::now().duration_since(begin);
    println!("Elapsed: {:?}", elapsed);
    assert!(elapsed.as_millis() >= 500 && elapsed.as_millis() <= 550);
}

#[test]
#[should_panic]
fn test_overflow_2_sync_threadsafe() {
    let rate_limiter = SyncLeakyBucket::builder()
        .max(5)
        .tokens(5)
        .refill_amount(1)
        .refill_interval(Duration::from_millis(100))
        .build();

    let begin = Instant::now();

    rate_limiter.acquire(10);

    let elapsed = Instant::now().duration_since(begin);
    println!("Elapsed: {:?}", elapsed);
    assert!(elapsed.as_millis() >= 500 && elapsed.as_millis() <= 550);
}
