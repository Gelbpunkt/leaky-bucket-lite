use leaky_bucket_lite::{sync_threadsafe::LeakyBucket as SyncLeakyBucket, LeakyBucket};

use std::time::{Duration, Instant};

#[tokio::test]
async fn test_issue5_a() {
    let rate_limiter = LeakyBucket::builder()
        .refill_amount(1)
        .refill_interval(Duration::from_millis(100))
        .build();

    let begin = Instant::now();

    for _ in 0..10 {
        rate_limiter.acquire_one().await;
    }

    let elapsed = Instant::now().duration_since(begin);
    println!("Elapsed: {:?}", elapsed);
    assert!((elapsed.as_secs_f64() - 1.).abs() < 0.1);
}

#[tokio::test]
async fn test_issue5_b() {
    let rate_limiter = LeakyBucket::builder()
        .refill_amount(1)
        .refill_interval(Duration::from_secs(2))
        .build();

    let begin = Instant::now();

    for _ in 0..2 {
        rate_limiter.acquire_one().await;
    }

    let elapsed = Instant::now().duration_since(begin);
    println!("Elapsed: {:?}", elapsed);
    // once per 2 seconds => 4 seconds for 2 permits
    assert!((elapsed.as_secs_f64() - 4.).abs() < 0.1);
}

#[tokio::test]
async fn test_issue5_c() {
    let start = Instant::now();
    let rate_limiter = LeakyBucket::builder()
        .refill_amount(5)
        .tokens(0)
        .refill_interval(Duration::from_secs(2))
        .build();

    tokio::time::sleep(Duration::from_secs(3)).await;
    rate_limiter.acquire(7).await;

    let elapsed = Instant::now().duration_since(start);
    println!("Elapsed in c: {:?}", elapsed);
    assert!((elapsed.as_secs_f64() - 4.).abs() < 0.1);
}

#[test]
fn test_issue5_a_sync_threadsafe() {
    let rate_limiter = SyncLeakyBucket::builder()
        .refill_amount(1)
        .refill_interval(Duration::from_millis(100))
        .build();

    let begin = Instant::now();

    for _ in 0..10 {
        rate_limiter.acquire_one();
    }

    let elapsed = Instant::now().duration_since(begin);
    println!("Elapsed: {:?}", elapsed);
    assert!((elapsed.as_secs_f64() - 1.).abs() < 0.1);
}

#[test]
fn test_issue5_b_sync_threadsafe() {
    let rate_limiter = SyncLeakyBucket::builder()
        .refill_amount(1)
        .refill_interval(Duration::from_secs(2))
        .build();

    let begin = Instant::now();

    for _ in 0..2 {
        rate_limiter.acquire_one();
    }

    let elapsed = Instant::now().duration_since(begin);
    println!("Elapsed: {:?}", elapsed);
    // once per 2 seconds => 4 seconds for 2 permits
    assert!((elapsed.as_secs_f64() - 4.).abs() < 0.1);
}

#[tokio::test]
async fn test_issue5_c_sync_threadsafe() {
    let start = Instant::now();
    let rate_limiter = SyncLeakyBucket::builder()
        .refill_amount(5)
        .tokens(0)
        .refill_interval(Duration::from_secs(2))
        .build();

    std::thread::sleep(Duration::from_secs(3));
    rate_limiter.acquire(7);

    let elapsed = Instant::now().duration_since(start);
    println!("Elapsed in c: {:?}", elapsed);
    assert!((elapsed.as_secs_f64() - 4.).abs() < 0.1);
}
