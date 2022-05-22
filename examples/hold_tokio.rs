//! Example of "holding" a slot open until some related work is complete.
//!
//! Note in the output of the example that while the long-running Stream B items are in progress,
//! that Stream A items are still throttled. In this scenario, a throttle slot remains "held"
//! during the entire duration of a stream item, not just during its creation.
//!
//! A typical use case for this is in construction of complex streams where you want to limit the
//! number of concurrent requests to a web service, AND not open new requests until long-running
//! downloads of response bodies complete, etc.

#![cfg(feature = "timer-tokio")]

use futures::prelude::*;
use futures::stream;
use std::time::{Duration, Instant};
use stream_throttle::{ThrottlePool, ThrottleRate, ThrottledStream};

#[tokio::main]
async fn main() {
	let rate = ThrottleRate::new(2, Duration::new(1, 0));
	println!("{:?}", rate);
	println!("Stream B span duration: 5s");

	let pool = ThrottlePool::new(rate);

	let (tx, mut rx) = futures::channel::mpsc::unbounded::<(String, Instant)>();

	// a stream that can technically produce all of its items immediately, but is throttled
	let stream_a = stream::iter(0..50).throttle(pool.clone()).map({
		let tx = tx.clone();
		move |i| {
			tx.unbounded_send((format!("stream A[{:2}] item", i + 1), Instant::now()))
				.unwrap()
		}
	});

	// a more complex stream where not only do we want to wait for a free slot in the throttle pool,
	// but we also want to "hold" that slot until some related work is completed.
	// Rather than use .throttle() on the stream, we use pool.queue_with_hold() directly.
	let stream_b = stream::iter(0..5).then({
		let tx = tx.clone();
		let pool = pool.clone();
		move |i| {
			let pool = pool.clone();
			let tx = tx.clone();
			async move {
				tx.unbounded_send((format!("stream B[{:2}] item start", i + 1), Instant::now()))
					.unwrap();

				// wait for a free slot, and then get a hold handle
				// The hold will remain in place until the handle is released or dropped.
				let hold = pool.queue_with_hold().await;

				// do some work while we have the hold
				tokio::time::sleep(Duration::from_secs(5)).await;

				// release the hold
				hold.release();

				tx.unbounded_send((format!("stream B[{:2}] item end", i + 1), Instant::now()))
					.unwrap();
			}
		}
	});

	// drop the original tx, so that the rx will close when all stream tx's are dropped
	drop(tx);

	tokio::spawn(stream_a.for_each(|_| async {}));
	tokio::spawn(stream_b.for_each(|_| async {}));

	let mut last_instant = Instant::now();

	while let Some((message, instant)) = rx.next().await {
		println!(
			"({:23}) delayed: {:10}ms",
			message,
			instant.duration_since(last_instant).as_millis()
		);

		last_instant = instant;
	}
}
