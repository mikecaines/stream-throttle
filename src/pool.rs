use super::ThrottleRate;
use error::{Error};
use futures::stream;
use futures::{Future, Stream};
use futures_timer;
use futures_util::compat::{Compat};
use futures_util::FutureExt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A clonable object which is used to throttle one or more streams, according to a shared rate.
#[derive(Clone)]
pub struct ThrottlePool {
	inner: Arc<ThrottlePoolInner>,
}

#[derive(Debug)]
struct ThrottlePoolInner {
	rate_duration: Duration,
	slots: Vec<Mutex<Instant>>, // expiry times, one for each item in rate.count
}

impl ThrottlePool {
	pub fn new(rate: ThrottleRate) -> Self {
		let mut slots = Vec::with_capacity(rate.count());
		for _ in 0..rate.count() {
			slots.push(Mutex::new(Instant::now() - rate.duration()));
		}

		Self {
			inner: Arc::new(ThrottlePoolInner {
				rate_duration: rate.duration(),
				slots,
			}),
		}
	}

	/// Produces a future which will resolve once the pool has an available slot.
	///
	/// Each `Throttled` stream will call this method during polling, once for each item the
	/// underlying stream produces. These futures are driven to completion by polling the
	/// `Throttled` stream. In the process, these futures will drive the `ThrottlePool`,
	/// freeing up slots.
	/// Note: Error is never returned.
	pub fn queue(&self) -> impl Future<Item = (), Error = Error> {
		stream::repeat(())
			.and_then({
				let inner = self.inner.clone();
				move |_| {
					let now = Instant::now();
					let mut sleep = inner.rate_duration;

					for slot in &inner.slots {
						if let Ok(mut slot) = slot.try_lock() {
							// if the slot's instant is in the past
							if *slot <= now {
								// the slot is expired/free
								// set the slot's new expiry instant to be now + rate.duration
								*slot = now + inner.rate_duration;
								return Ok(None); // let the stream end
							} else {
								// if the slot's expiry is the earliest one we've encountered, use it
								sleep = ::std::cmp::min(*slot - now, sleep);
							}
						}
						// else we couldn't lock the mutex
						else {
							// just let the stream iterate one item, and try again
							return Ok(Some(Duration::from_secs(0)));
						}
					}

					if log_enabled!(::log::Level::Trace) {
						trace!("Sleeping for {:?}", sleep);
					}

					Ok(Some(sleep))
				}
			})
			.take_while(|sleep| Ok(sleep.is_some()))
			.and_then({
				move |sleep| {
					// sleep for the required duration
					let delay =
						futures_timer::Delay::new(sleep.unwrap_or_else(|| Duration::from_secs(0)))
							.map(|o| Ok::<(), Error>(o));
					Compat::new(delay)
				}
			})
			.for_each(|_| Ok(()))
			.from_err()
	}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_complete() {
		let pool = ThrottlePool::new(ThrottleRate::new(1, Duration::from_millis(1)));

		let work = pool.queue().map(|_| 1);

		let all = tokio::runtime::current_thread::block_on_all(work);

		assert_eq!(all.unwrap(), 1)
    }

    #[test]
    fn can_complete_multiple() {
		let pool = ThrottlePool::new(ThrottleRate::new(1, Duration::from_millis(1)));

		let work1 = pool.queue().map(|_| 1);
		let work2 = pool.queue().map(|_| 2);
		let work3 = pool.queue().map(|_| 3);
		let work4 = pool.queue().map(|_| 4);

		let work = work1.join4(work2, work3, work4);

		let all = tokio::runtime::current_thread::block_on_all(work);

		assert_eq!(all.unwrap(), (1,2,3,4))
    }

    #[test]
    fn delay_does_work() {
		let delay_ms = 50;
		let pool = ThrottlePool::new(ThrottleRate::new(1, Duration::from_millis(delay_ms)));

		let work1 = pool.queue().map(|_| 1);
		let work2 = pool.queue().map(|_| 2);
		let work3 = pool.queue().map(|_| 3);
		let work4 = pool.queue().map(|_| 4);
		let work = work1.join4(work2, work3, work4);

		let work_start = Instant::now();
		let all = tokio::runtime::current_thread::block_on_all(work);
		let work_finish = Instant::now();

		let time_taken_ms = work_finish.duration_since(work_start).as_millis() as u64;

		assert!(time_taken_ms > (3 * delay_ms));
		assert!(time_taken_ms < (4 * delay_ms))
    }
}
