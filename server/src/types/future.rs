
use std::sync::Mutex;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use component::event::TimerEvent;

use tokio;
use futures::Future;

pub struct FutureDispatcher {
	channel: Mutex<Sender<TimerEvent>>,
}

/// Allow spawning of futures on the tokio event
/// loop, these futures can communicate back with
/// the main game loop via [`TimerEvent`]s
impl FutureDispatcher {
	pub fn new(channel: Sender<TimerEvent>) -> Self {
		Self {
			channel: Mutex::new(channel)
		}
	}

	/// Runs the function after the 
	pub fn run_delayed<F: 'static>(
		&self,
		dur: Duration, 
		fun: F
	)
	where 
		F: Send + FnOnce(Instant) -> Option<TimerEvent>
	{
		let channel = self.channel.lock().unwrap().clone();
		let instant = Instant::now() + dur;

		tokio::spawn(
			tokio::timer::Delay::new(instant)
				.map_err(|_| {})
				.and_then(move |_| -> Result<(), ()> {
					let retval = fun(instant);

					if retval.is_some() {
						channel.send(retval.unwrap())
							.map_err(|e| {
								error!("Channel send error: {:?}", e);

								e
							})
							.err();
					}

					Ok(())
				})
		);
	}
}

