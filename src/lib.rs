//! Provides a
//! [`Stream`](../futures/stream/trait.Stream.html) combinator, to limit the rate at which
//! items are produced. Multiple streams can be throttled according to a shared rate.

#[macro_use]
extern crate log;
#[macro_use]
extern crate futures;
extern crate tokio_timer;
#[macro_use]
extern crate failure_derive;
extern crate failure;

pub mod error;

use error::{Error, ErrorKind};
use failure::Fail;
use futures::stream;
use futures::{Async, Future, Poll, Stream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio_timer::Timer;

/// Provides a `throttle()` method on all `Stream`'s.
pub trait ThrottledStream {
	/// Returns a new stream, which throttles items from the original stream, according to the
	/// rate defined by `pool`.
	fn throttle(self, pool: ThrottlePool) -> Throttled<Self>
	where
		Self: Stream + Sized,
	{
		Throttled {
			stream: self,
			pool,
			pending: None,
		}
	}
}

impl<T: Stream> ThrottledStream for T {}

/// A stream combinator which throttles its elements via a shared `ThrottlePool`.
///
/// This structure is produced by the `ThrottledStream::throttle()` method.
#[must_use = "streams do nothing unless polled"]
pub struct Throttled<S>
where
	S: Stream + 'static,
{
	stream: S,
	pool: ThrottlePool,

	// the first Option layer represents a pending item for this Throttled stream
	// The second Option layer contains a future produced by ThrottlePool::queue()
	pending: Option<Option<Box<Future<Item = (), Error = S::Error>>>>,
}

impl<S> Stream for Throttled<S>
where
	S: Stream,
{
	type Item = S::Item;
	type Error = S::Error;

	/// Calls ThrottlePool::queue() to get slot in the throttle queue, waits for it to resolve, and
	/// then polls the underlying stream for an item, and produces it.
	fn poll(&mut self) -> Poll<Option<S::Item>, S::Error> {
		// if we don't already have a pending item, get one
		if self.pending.is_none() {
			self.pending = Some(Some(Box::new(
				self.pool
					.queue()
					.map_err(|e| panic!("ThrottlePool::queue() failed: {}", e)),
			)));
		}
		assert!(self.pending.is_some());

		// if we have a queue() future, we are still waiting for the queue slot to expire
		if self.pending.as_mut().unwrap().is_some() {
			// poll the queue slot future, and remove it once it resolves
			try_ready!(self.pending.as_mut().unwrap().as_mut().unwrap().poll());
			self.pending.as_mut().unwrap().take();
		}

		// poll the underlying stream until we get an item, or the stream ends
		match try_ready!(self.stream.poll()) {
			Some(item) => {
				self.pending = None;
				Ok(Async::Ready(Some(item)))
			}

			None => Ok(Async::Ready(None)),
		}
	}
}

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
		let mut slots = Vec::with_capacity(rate.count);
		for _ in 0..rate.count {
			slots.push(Mutex::new(Instant::now() - rate.duration));
		}

		Self {
			inner: Arc::new(ThrottlePoolInner {
				timer,
				rate_duration: rate.duration,
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

/// Defines the the throttle rate.
///
/// e.g. 3 items per-second.
#[derive(Copy, Clone, Debug)]
pub struct ThrottleRate {
	count: usize,
	duration: Duration,
}

impl ThrottleRate {
	pub fn new(count: usize, duration: Duration) -> Self {
		assert!(count > 0);
		assert!(duration > Duration::from_millis(0));

		Self { count, duration }
	}

	pub fn count(&self) -> usize {
		self.count
	}

	pub fn duration(&self) -> Duration {
		self.duration
	}
}
