use shrev::*;
use specs::*;
use types::*;

use protocol::client::Chat;
use protocol::server::{ChatPublic, ServerPacket, Error};
use protocol::{to_bytes, ErrorType};
use websocket::OwnedMessage;

use component::flag::{IsChatThrottled, IsChatMuted};

pub struct ChatHandler {
	reader: Option<ReaderId<(ConnectionId, Chat)>>,
}

#[derive(SystemData)]
pub struct ChatHandlerData<'a> {
	channel: Read<'a, EventChannel<(ConnectionId, Chat)>>,
	conns: Read<'a, Connections>,

	throttled: ReadStorage<'a, IsChatThrottled>,
	muted: ReadStorage<'a, IsChatMuted>,
}

impl ChatHandler {
	pub fn new() -> Self {
		Self { reader: None }
	}
}

impl<'a> System<'a> for ChatHandler {
	type SystemData = ChatHandlerData<'a>;

	fn setup(&mut self, res: &mut Resources) {
		self.reader = Some(
			res.fetch_mut::<EventChannel<(ConnectionId, Chat)>>()
				.register_reader(),
		);

		Self::SystemData::setup(res);
	}

	fn run(&mut self, data: Self::SystemData) {
		for evt in data.channel.read(self.reader.as_mut().unwrap()) {
			let player = match data.conns.0.get(&evt.0) {
				Some(data) => match data.player {
					Some(player) => player,
					None => continue,
				},
				None => continue,
			};

			if data.muted.get(player).is_some() { continue; }
			if data.throttled.get(player).is_some() { 
				data.conns.send_to(evt.0, OwnedMessage::Binary(
					to_bytes(&ServerPacket::Error(Error {
						error: ErrorType::ChatThrottled
					})).unwrap()
				));
				continue;
			}

			let chat = ChatPublic {
				id: player,
				text: evt.1.text.clone(),
			};

			data.conns.send_to_all(OwnedMessage::Binary(
				to_bytes(&ServerPacket::ChatPublic(chat)).unwrap(),
			));
		}
	}
}

use dispatch::SystemInfo;
use handlers::OnCloseHandler;

impl SystemInfo for ChatHandler {
	type Dependencies = OnCloseHandler;

	fn new() -> Self {
		Self::new()
	}

	fn name() -> &'static str {
		concat!(module_path!(), "::", line!())
	}
}
