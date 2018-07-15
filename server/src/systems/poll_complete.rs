use metrics::*;
use specs::prelude::*;
use std::time::Instant;
use tokio::prelude::Sink;
use types::connection::{Message, MessageInfo};
use types::*;

use websocket::OwnedMessage;

use std::mem;
use std::sync::mpsc::{channel, Receiver};

pub struct PollComplete {
	channel: Receiver<Message>,
}

#[derive(SystemData)]
pub struct PollCompleteData<'a> {
	conns: Write<'a, Connections>,
	metrics: ReadExpect<'a, MetricsHandler>,

	associated: ReadStorage<'a, AssociatedConnection>,
	teams: ReadStorage<'a, Team>,
}

impl PollComplete {
	pub fn new(channel: Receiver<Message>) -> Self {
		Self { channel }
	}
}

impl PollComplete {
	fn send_to_connection<'a>(
		conns: &mut Write<'a, Connections>,
		id: ConnectionId,
		msg: OwnedMessage,
	) {
		match conns.0.get_mut(&id) {
			Some(ref mut conn) => {
				Connections::send_sink(&mut conn.sink, msg);
			}
			// The connection probably closed,
			// do nothing
			None => trace!(
					target: "server",
					"Tried to send message to closed connection {:?}",
					id
			),
		}
	}
}

impl<'a> System<'a> for PollComplete {
	type SystemData = PollCompleteData<'a>;

	fn run(&mut self, data: Self::SystemData) {
		let mut conns = data.conns;
		let metrics = data.metrics;
		let associated = data.associated;
		let teams = data.teams;

		let start = Instant::now();
		let mut cnt = 0;
		while let Ok(msg) = self.channel.try_recv() {
			cnt += 1;

			match msg.info {
				MessageInfo::ToConnection(id) => Self::send_to_connection(&mut conns, id, msg.msg),
				MessageInfo::ToTeam(player) => {
					let player_team = *teams.get(player).unwrap();

					(&associated, &teams)
						.join()
						.filter(|(_, team)| **team == player_team)
						.for_each(|(associated, _)| {
							Self::send_to_connection(&mut conns, associated.0, msg.msg.clone());
						});
				}
				MessageInfo::ToVisible(_player) => {
					// TODO: Implement this properly
					(&associated).join().for_each(|associated| {
						Self::send_to_connection(&mut conns, associated.0, msg.msg.clone());
					});
				}
			}
		}

		metrics.count("packets-sent", cnt).unwrap();

		for conn in conns.iter_mut() {
			conn.sink
				.poll_complete()
				.map_err(|e| {
					info!("poll_complete failed with error {:?}", e);
				})
				.err();
		}

		metrics
			.time_duration("poll-complete", Instant::now() - start)
			.err();
	}
}

use dispatch::SystemInfo;
use std::any::Any;

impl SystemInfo for PollComplete {
	type Dependencies = ();

	fn name() -> &'static str {
		concat!(module_path!(), "::", line!())
	}

	fn new() -> Self {
		unimplemented!();
	}

	fn new_args(mut a: Box<Any>) -> Self {
		let r = a.downcast_mut::<Receiver<Message>>().unwrap();
		// Replace the channel within the box with a
		// dummy one, which will be dropped immediately
		// anyway
		Self::new(mem::replace(r, channel().1))
	}
}
