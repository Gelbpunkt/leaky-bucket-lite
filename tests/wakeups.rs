use leaky_bucket_lite::Builder;

use tokio::time::{self, Duration};

#[tokio::test]
async fn test_concurrent_rate_limited() {
    let interval = Duration::from_millis(20);

    let leaky = Builder::new()
        .tokens(0.0)
        .max(10.0)
        .refill_amount(1.0)
        .refill_interval(interval)
        .build();

    let mut one_wakeups = 0;

    let one = async {
        loop {
            leaky.acquire(1.0).await;
            one_wakeups += 1;
        }
    };

    let mut two_wakeups = 0u32;

    let two = async {
        loop {
            leaky.acquire(1.0).await;
            two_wakeups += 1;
        }
    };

    let delay = time::sleep(Duration::from_millis(200));

    let task = async {
        tokio::select! {
            _ = one => {},
            _ = two => {},
        }
    };

    tokio::select! {
        _ = task => {},
        _ = delay => {},
    }

    let total = one_wakeups + two_wakeups;

    assert!(total > 5 && total < 15);
}
