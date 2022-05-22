use super::ThrottleRate;
use futures::channel::oneshot::{Receiver, Sender};
use futures::Future;
use log::{log_enabled, trace};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A clonable object which is used to throttle one or more streams, according to a shared rate.
#[derive(Clone)]
pub struct ThrottlePool {
	inner: Arc<ThrottlePoolInner>,
}

impl ThrottlePool {
	pub fn new(rate: ThrottleRate) -> Self {
		let mut slots = Vec::with_capacity(rate.count());
		for _ in 0..rate.count() {
			slots.push(Mutex::new(Slot {
				wait_until: Instant::now() - rate.duration(),
				hold: None,
			}));
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
		let queue = self.queue_with_hold();
		async move {
			// just wait for an available slot, and immediately drop the hold handle
			queue.await;
		}
	}

	/// Similar to queue(), but also returns a handle that will "hold" the slot until released.
	///
	/// The hold will be released automatically once the hold handle is dropped.
	pub fn queue_with_hold(&self) -> impl Future<Output = HoldHandle> {
		let inner = self.inner.clone();
		async move {
			// the "outer" loop which will only end via return
			loop {
				let now = Instant::now();
				let mut sleep = inner.rate_duration;

				for slot in &inner.slots {
					if let Ok(mut slot) = slot.try_lock() {
						// if the slot's instant is in the past
						if slot.wait_until <= now {
							// if the slot already has a hold receiver
							if let Some(rx) = &mut slot.hold {
								// if the hold has been released
								if rx.try_recv().is_err() {
									// the slot is expired/free
									// set the slot's new expiry instant to be now + rate.duration
									slot.wait_until = now + inner.rate_duration;

									// clear the slot's hold receiver
									slot.hold = None;
								}
								// else the hold is still in place
								else {
									// yield to the outer loop
									sleep = Duration::from_secs(0);
									break;
								}
							}
							// else the slot does not have a hold receiver yet
							else {
								// set the slot's hold receiver

								let (tx, rx) = futures::channel::oneshot::channel();
								slot.hold = Some(rx);

								// let the stream end
								return HoldHandle { tx: Some(tx) };
							}
						}
						// else the slot's expiry is in the future
						else {
							// if the slot's expiry is the earliest one we've encountered, use it
							sleep = std::cmp::min(slot.wait_until - now, sleep);
						}
					}
					// else we couldn't lock the mutex
					else {
						// yield to the outer loop
						sleep = Duration::from_secs(0);
						break;
					}
				}

				if log_enabled!(log::Level::Trace) {
					trace!("Sleeping for {:?}", sleep);
				}

				delay_for(sleep).await;
			}
		}
	}
}

#[derive(Debug)]
struct ThrottlePoolInner {
	rate_duration: Duration,
	slots: Vec<Mutex<Slot>>, // expiry times, one for each item in rate.count
}

#[derive(Debug)]
struct Slot {
	wait_until: Instant,
	hold: Option<Receiver<()>>,
}

pub struct HoldHandle {
	// when the QueueHandle is dropped, so is its tx, which notifies the rx of cancellation
	tx: Option<Sender<()>>,
}

impl HoldHandle {
	pub fn release(mut self) {
		self.tx.take();
	}
}

#[cfg(feature = "timer-tokio")]
fn delay_for(dur: Duration) -> impl Future<Output = ()> {
	tokio::time::sleep(dur)
}

#[cfg(feature = "timer-futures-timer")]
fn delay_for(dur: Duration) -> impl Future<Output = ()> {
	futures_timer::Delay::new(dur)
}
