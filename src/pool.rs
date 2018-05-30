use super::ThrottleRate;
use error::{Error, ErrorKind};
use failure::Fail;
use futures::stream;
use futures::{Future, Stream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio_timer::Timer;

/// A clonable object which is used to throttle one or more streams, according to a shared rate.
#[derive(Clone)]
pub struct ThrottlePool {
	inner: Arc<ThrottlePoolInner>,
}

#[derive(Debug)]
struct ThrottlePoolInner {
	timer: Timer,
	rate_duration: Duration,
	slots: Vec<Mutex<Instant>>, // expiry times, one for each item in rate.count
}

impl ThrottlePool {
	pub fn new(rate: ThrottleRate, timer: Timer) -> Self {
		let mut slots = Vec::with_capacity(rate.count());
		for _ in 0..rate.count() {
			slots.push(Mutex::new(Instant::now() - rate.duration()));
		}

		Self {
			inner: Arc::new(ThrottlePoolInner {
				timer,
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
	pub fn queue(&self) -> impl Future<Item = (), Error = Error> {
		let inner = self.inner.clone();
		let inner2 = self.inner.clone();

		stream::repeat(())
			.and_then(move |_| {
				let now = Instant::now();
				let mut sleep = inner.rate_duration;

				for mut slot in &inner.slots {
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
			})
			.take_while(|sleep| Ok(sleep.is_some()))
			.and_then(move |sleep| {
				// sleep for the required duration
				inner2
					.timer
					.sleep(sleep.unwrap_or_else(|| Duration::from_secs(0)))
					.map_err(|e| e.context(ErrorKind::Timer("queue future could not sleep")))
			})
			.for_each(|_| Ok(()))
			.from_err()
	}
}
