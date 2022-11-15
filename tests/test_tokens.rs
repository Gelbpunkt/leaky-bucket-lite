use leaky_bucket_lite::{sync_threadsafe::LeakyBucket as SyncLeakyBucket, LeakyBucket};
use tokio::time::sleep;

use std::{
    future::ready,
    time::{Duration, Instant},
};

#[tokio::test]
async fn test_tokens() {
    let rate_limiter = LeakyBucket::builder()
        .max(5)
        .tokens(5)
        .refill_amount(1)
        .refill_interval(Duration::from_millis(100))
        .build();

    assert_eq!(rate_limiter.tokens().await, 5);

    for i in 0..5 {
        assert_eq!(rate_limiter.tokens().await, 5 - i);
        rate_limiter.acquire_one().await;
    }

    assert_eq!(rate_limiter.tokens().await, 0);
    rate_limiter.acquire_one().await;
    assert_eq!(rate_limiter.tokens().await, 0);
}

#[test]
fn test_tokens_sync_threadsafe() {
    let rate_limiter = SyncLeakyBucket::builder()
        .max(5)
        .tokens(5)
        .refill_amount(1)
        .refill_interval(Duration::from_millis(100))
        .build();

    assert_eq!(rate_limiter.tokens(), 5);

    for i in 0..5 {
        assert_eq!(rate_limiter.tokens(), 5 - i);
        rate_limiter.acquire_one();
    }

    assert_eq!(rate_limiter.tokens(), 0);
    rate_limiter.acquire_one();
    assert_eq!(rate_limiter.tokens(), 0);
}

#[tokio::test]
async fn test_concurrent_tokens() {
    let rate_limiter = LeakyBucket::builder()
        .max(5)
        .tokens(0)
        .refill_amount(1)
        .refill_interval(Duration::from_millis(100))
        .build();

    let rl = rate_limiter.clone();
    let begin = Instant::now();

    assert_eq!(rate_limiter.tokens().await, 0);

    let (tx, rx) = tokio::sync::oneshot::channel();

    tokio::spawn(async move {
        let fut = rl.acquire(5);
        tokio::pin!(fut);
        // poll the future once, to ensure the lock is acquired
        tokio::select! {
          biased;

          _ = &mut fut => (),
          _ = ready(()) => {
            tx.send(()).unwrap();
          }
        };
        // poll the future to completion, which releases the lock
        fut.await;
    });

    rx.await.unwrap();

    tokio::select! {
      _ = rate_limiter.tokens() => panic!("should not complete"),
      _ = sleep(Duration::from_millis(400)) => {}
    };

    assert_eq!(rate_limiter.tokens().await, 0);
    assert!((begin.elapsed().as_millis() - 500) < 50);
}
