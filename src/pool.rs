use super::ThrottleRate;
use futures::{Future, StreamExt};
use log::{log_enabled, trace};
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
	pub fn queue(&self) -> impl Future<Output = ()> {
		futures::stream::repeat(())
			.map({
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
								return None; // let the stream end
							} else {
								// if the slot's expiry is the earliest one we've encountered, use it
								sleep = ::std::cmp::min(*slot - now, sleep);
							}
						}
						// else we couldn't lock the mutex
						else {
							// just let the stream iterate one item, and try again
							return Some(Duration::from_secs(0));
						}
					}

					if log_enabled!(::log::Level::Trace) {
						trace!("Sleeping for {:?}", sleep);
					}

					Some(sleep)
				}
			})
			.take_while(|sleep| futures::future::ready(sleep.is_some()))
			.then({
				move |sleep| {
					// sleep for the required duration
					tokio::time::delay_for(sleep.unwrap_or_else(|| Duration::from_secs(0)))
				}
			})
			.for_each(|_| futures::future::ready(()))
	}
}
