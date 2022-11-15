use leaky_bucket_lite::LeakyBucket;
use tokio::time::{sleep, Duration, Instant};

#[tokio::test]
async fn test_next_refill() {
    let rate_limiter = LeakyBucket::builder()
        .refill_amount(5)
        .max(5)
        .tokens(0)
        .refill_interval(Duration::from_secs(2))
        .build();

    let next_refill = rate_limiter.next_refill();
    let until = next_refill.await.duration_since(Instant::now());
    assert!((until.as_secs_f64() - 2.).abs() < 0.1);

    sleep(Duration::from_secs(3)).await;

    let next_refill = rate_limiter.next_refill();
    let until = next_refill.await.duration_since(Instant::now());
    assert!((until.as_secs_f64() - 1.).abs() < 0.1);

    rate_limiter.acquire(5).await;

    let next_refill = rate_limiter.next_refill();
    let until = next_refill.await.duration_since(Instant::now());
    assert!((until.as_secs_f64() - 1.).abs() < 0.1);
}
