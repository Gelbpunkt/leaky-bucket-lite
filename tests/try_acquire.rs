use leaky_bucket_lite::TryAcquireError;
use std::{
    future::ready,
    time::{Duration, Instant},
};

#[tokio::test]
async fn test_tokio() {
    use leaky_bucket_lite::LeakyBucket;

    let rate_limiter = LeakyBucket::builder()
        .max(5)
        .tokens(5)
        .refill_amount(1)
        .refill_interval(Duration::from_millis(100))
        .build();

    let begin = Instant::now();

    assert!(matches!(rate_limiter.try_acquire(5), Ok(())));
    assert_is_insufficient_tokens(
        rate_limiter.try_acquire(1),
        begin,
        Duration::from_millis(100),
    );

    std::thread::sleep(Duration::from_millis(100));

    assert_is_insufficient_tokens(
        rate_limiter.try_acquire(2),
        begin,
        Duration::from_millis(200),
    );
    assert!(matches!(rate_limiter.try_acquire(1), Ok(())));

    rate_limiter.acquire(1).await;
    assert_duration_equal(begin.elapsed(), Duration::from_millis(200));

    let (tx, rx) = tokio::sync::oneshot::channel();
    tokio::spawn({
        let rate_limiter = rate_limiter.clone();
        async move {
            let fut = rate_limiter.acquire(1);
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
        }
    });
    // wait until the lock is acquired
    rx.await.unwrap();
    assert_is_locked(rate_limiter.try_acquire(1));

    rate_limiter.acquire(1).await;
    assert_duration_equal(begin.elapsed(), Duration::from_millis(400));

    assert_is_insufficient_tokens(
        rate_limiter.try_acquire(1),
        begin,
        Duration::from_millis(500),
    );
}

#[test]
fn test_sync_threadsafe() {
    use leaky_bucket_lite::sync_threadsafe::LeakyBucket;

    let rate_limiter = LeakyBucket::builder()
        .max(5)
        .tokens(5)
        .refill_amount(1)
        .refill_interval(Duration::from_millis(100))
        .build();

    let begin = Instant::now();

    assert!(matches!(rate_limiter.try_acquire(5), Ok(())));
    assert_is_insufficient_tokens(
        rate_limiter.try_acquire(1),
        begin,
        Duration::from_millis(100),
    );

    std::thread::sleep(Duration::from_millis(100));

    assert_is_insufficient_tokens(
        rate_limiter.try_acquire(2),
        begin,
        Duration::from_millis(200),
    );
    assert!(matches!(rate_limiter.try_acquire(1), Ok(())));

    rate_limiter.acquire(1);
    assert_duration_equal(begin.elapsed(), Duration::from_millis(200));

    let (tx, rx) = std::sync::mpsc::channel();
    {
        let rate_limiter = rate_limiter.clone();
        std::thread::spawn(move || {
            tx.send(()).unwrap();
            rate_limiter.acquire(1);
        });
    };

    // wait until the thread is running
    rx.recv().unwrap();
    // give the thread some time to acquire the lock
    std::thread::sleep(Duration::from_millis(5));
    assert_is_locked(rate_limiter.try_acquire(1));

    rate_limiter.acquire(1);
    assert_duration_equal(begin.elapsed(), Duration::from_millis(400));

    assert_is_insufficient_tokens(
        rate_limiter.try_acquire(1),
        begin,
        Duration::from_millis(500),
    );
}

#[test]
fn test_sync() {
    use leaky_bucket_lite::sync_threadsafe::LeakyBucket;

    let rate_limiter = LeakyBucket::builder()
        .max(5)
        .tokens(5)
        .refill_amount(1)
        .refill_interval(Duration::from_millis(100))
        .build();

    let begin = Instant::now();

    let assert_is_insufficient_tokens = |r: Result<(), TryAcquireError>, d| match r {
        Ok(()) => panic!("try_acquire should have failed"),
        Err(e) => {
            assert!(e.is_insufficient_tokens());
            assert!(!e.is_locked());
            assert_duration_equal(e.target_time().unwrap().duration_since(begin), d);
        }
    };

    assert!(matches!(rate_limiter.try_acquire(5), Ok(())));
    assert_is_insufficient_tokens(rate_limiter.try_acquire(1), Duration::from_millis(100));

    std::thread::sleep(Duration::from_millis(100));

    assert_is_insufficient_tokens(rate_limiter.try_acquire(2), Duration::from_millis(200));
    assert!(matches!(rate_limiter.try_acquire(1), Ok(())));

    rate_limiter.acquire(1);
    assert_duration_equal(begin.elapsed(), Duration::from_millis(200));
}

fn assert_duration_equal(d1: Duration, d2: Duration) {
    let diff = (d1.as_secs_f64() - d2.as_secs_f64()).abs();
    assert!(diff < 0.02, "{:?} differs from {:?} by {}s", d1, d2, diff);
}

fn assert_is_insufficient_tokens(r: Result<(), TryAcquireError>, begin: Instant, d: Duration) {
    match r {
        Ok(()) => panic!("try_acquire should have failed"),
        Err(e) => {
            assert!(e.is_insufficient_tokens());
            assert!(!e.is_locked());
            assert_duration_equal(e.target_time().unwrap().duration_since(begin), d);
        }
    };
}

fn assert_is_locked(r: Result<(), TryAcquireError>) {
    match r {
        Ok(()) => panic!("try_acquire should have failed"),
        Err(e) => {
            assert!(e.is_locked());
            assert!(!e.is_insufficient_tokens());
            assert_eq!(None, e.target_time());
        }
    };
}
