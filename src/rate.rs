use std::time::Duration;

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
