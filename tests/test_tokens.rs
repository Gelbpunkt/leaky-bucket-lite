use leaky_bucket_lite::LeakyBucket;

use std::time::Duration;

#[tokio::test]
async fn test_tokens() {
    let rate_limiter = LeakyBucket::builder()
        .max(5)
        .tokens(5)
        .refill_amount(1)
        .refill_interval(Duration::from_millis(100))
        .build();

    assert_eq!(rate_limiter.tokens().await.expect("No reason to fail"), 5);

    for i in 0..5 {
        assert_eq!(
            rate_limiter.tokens().await.expect("No reason to fail"),
            5 - i
        );
        rate_limiter.acquire_one().await.expect("No reason to fail");
    }

    assert_eq!(rate_limiter.tokens().await.expect("No reason to fail"), 0);
    rate_limiter.acquire_one().await.expect("No reason to fail");
    assert_eq!(rate_limiter.tokens().await.expect("No reason to fail"), 0);
}
