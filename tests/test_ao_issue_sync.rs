#[test]
#[cfg(feature = "sync")]
fn test_ao_issue() {
    use leaky_bucket_lite::sync::LeakyBucket;
    use std::time::{Duration, Instant};

    let mut rate_limiter = LeakyBucket::builder()
        .refill_amount(1.0)
        .refill_interval(Duration::from_secs(2))
        .max(5.0)
        .tokens(5.0)
        .build();

    let begin = Instant::now();

    std::thread::sleep(Duration::from_secs(10));

    for _ in 0..10 {
        rate_limiter.acquire_one();
    }

    let elapsed = Instant::now().duration_since(begin);
    println!("Elapsed: {:?}", elapsed);
    assert!(elapsed.as_millis() >= 20000 && elapsed.as_millis() <= 20050);
}

#[test]
#[cfg(feature = "sync")]
fn test_ao_issue_2() {
    use leaky_bucket_lite::sync::LeakyBucket;
    use std::time::{Duration, Instant};

    let mut rate_limiter = LeakyBucket::builder()
        .refill_amount(1.0)
        .refill_interval(Duration::from_secs(2))
        .max(5.0)
        .tokens(5.0)
        .build();

    std::thread::sleep(Duration::from_secs(11));

    rate_limiter.acquire_one();
    let first_acquire = Instant::now();

    for _ in 0..4 {
        rate_limiter.acquire_one();
    }
    rate_limiter.acquire_one();
    let elapsed = Instant::now().duration_since(first_acquire);
    println!("Elapsed: {:?}", elapsed);
    assert!((elapsed.as_secs_f64() - 1.0).abs() < 0.1);
}
