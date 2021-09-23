use leaky_bucket_lite::{sync_threadsafe::LeakyBucket as SyncLeakyBucket, LeakyBucket};

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::channel,
        Arc,
    },
    time::{Duration, Instant},
};

/// Test that a bunch of threads spinning on a rate limiter refilling a
/// reasonable amount of tokens at a slowish rate reaches the given target.
#[tokio::test]
async fn test_rate_limit_target() {
    let rate_limiter = LeakyBucket::builder()
        .refill_amount(50.0)
        .refill_interval(Duration::from_millis(200))
        .build();

    let c = Arc::new(AtomicUsize::new(0));

    let start = Instant::now();

    let mut tasks = Vec::new();

    for _ in 0..100 {
        let rate_limiter = rate_limiter.clone();
        let c = c.clone();

        tasks.push(tokio::spawn(async move {
            while c.fetch_add(1, Ordering::SeqCst) < 500 {
                rate_limiter.acquire_one().await.unwrap();
            }
        }));
    }

    for t in tasks {
        t.await.unwrap();
    }

    let duration = Instant::now().duration_since(start);

    let diff = duration.as_millis() as f32 - 2000f32;
    assert!(
        diff.abs() < 10f32,
        "diff must be less than 10ms, but was {}ms",
        diff
    );
}

#[test]
fn test_rate_limit_target_sync_threadsafe() {
    // For sake of comparison, we need to pre-spawn the threads
    let mut senders = Vec::with_capacity(100);
    let mut threads = Vec::with_capacity(100);
    for _ in 0..100 {
        let (tx, rx) = channel::<(Arc<AtomicUsize>, SyncLeakyBucket)>();

        threads.push(std::thread::spawn(move || {
            let (c, rate_limiter) = rx.recv().unwrap();
            while c.fetch_add(1, Ordering::SeqCst) < 500 {
                rate_limiter.acquire_one();
            }
        }));
        senders.push(tx);
    }

    let rate_limiter = SyncLeakyBucket::builder()
        .refill_amount(50.0)
        .refill_interval(Duration::from_millis(200))
        .build();

    let c = Arc::new(AtomicUsize::new(0));

    let start = Instant::now();

    for sender in senders {
        let rate_limiter = rate_limiter.clone();
        let c = c.clone();

        sender.send((c, rate_limiter)).unwrap();
    }

    for t in threads {
        t.join().unwrap();
    }

    let duration = Instant::now().duration_since(start);

    let diff = duration.as_millis() as f32 - 2000f32;
    assert!(
        diff.abs() < 10f32,
        "diff must be less than 10ms, but was {}ms",
        diff
    );
}
