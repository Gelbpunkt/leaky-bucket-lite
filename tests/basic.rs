use leaky_bucket_lite::Builder;

use std::time::{Duration, Instant};

#[tokio::test]
async fn test_leaky_bucket() {
    let interval = Duration::from_millis(20);

    let leaky = Builder::new()
        .tokens(0.0)
        .max(10.0)
        .refill_amount(10.0)
        .refill_interval(interval)
        .build();

    let mut wakeups = 0u32;
    let mut duration = None;

    let test = async {
        let start = Instant::now();
        leaky.acquire(10.0).await;
        wakeups += 1;
        leaky.acquire(10.0).await;
        wakeups += 1;
        leaky.acquire(10.0).await;
        wakeups += 1;
        duration = Some(Instant::now().duration_since(start));
    };

    test.await;

    assert_eq!(3, wakeups);
    assert!(duration.expect("expected measured duration") > interval * 2);
}
