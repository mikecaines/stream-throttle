#![cfg(feature = "timer-futures-timer")]

use futures::prelude::*;
use futures::stream;
use std::time::{Duration, Instant};
use stream_throttle::{ThrottlePool, ThrottleRate, ThrottledStream};

#[async_std::main]
async fn main() {
	let rate = ThrottleRate::new(5, Duration::new(1, 0));
	println!("{:?}", rate);

	let pool = ThrottlePool::new(rate);

	let stream1 = {
		let mut count = 0;
		stream::repeat(())
			.throttle(pool.clone())
			.map(move |_| format!("{}", "stream 1"))
			.take_while(move |_| {
				let take = count < 10;
				count += 1;
				futures::future::ready(take)
			})
	};

	let stream2 = {
		let mut count = 0;
		stream::repeat(())
			.throttle(pool.clone())
			.map(move |_| format!("{}", "stream 2"))
			.take_while(move |_| {
				let take = count < 10;
				count += 1;
				futures::future::ready(take)
			})
	};

	let stream3 = {
		let mut count = 0;
		stream::repeat(())
			.throttle(pool.clone())
			.map(move |_| format!("{}", "stream 3"))
			.take_while(move |_| {
				let take = count < 10;
				count += 1;
				futures::future::ready(take)
			})
	};

	let mut last_instant = Instant::now();
	let mut index = 0;

	let work = futures::stream::select_all([stream1.boxed(), stream2.boxed(), stream3.boxed()])
		.for_each(move |name| {
			let now_instant = Instant::now();

			println!(
				"{:02} ({}) item delayed: {:?}",
				index,
				name,
				now_instant.duration_since(last_instant)
			);

			last_instant = now_instant;
			index += 1;

			futures::future::ready(())
		});

	work.await;
}
