use specs::*;

use component::channel::*;
use types::*;
use consts::timer::*;

use OwnedMessage;
use protocol::{to_bytes, ErrorType};
use protocol::client::Login;
use protocol::server::{Error, ServerPacket};

// Login needs write access to just
// about everything
#[derive(SystemData)]
pub struct LoginSystemData<'a> {
	pub conns: Read<'a, Connections>,
}

pub struct LoginFailed {
	reader: Option<OnTimerEventReader>,
}

impl LoginFailed {
	pub fn new() -> Self {
		Self { reader: None }
	}
}

impl<'a> System<'a> for LoginFailed {
	type SystemData = (Read<'a, OnTimerEvent>, LoginSystemData<'a>);

	fn setup(&mut self, res: &mut Resources) {
		self.reader = Some(res.fetch_mut::<OnTimerEvent>().register_reader());

		Self::SystemData::setup(res);
	}

	fn run(&mut self, (channel, data): Self::SystemData) {
		for evt in channel.read(self.reader.as_mut().unwrap()) {
			if evt.ty != *LOGIN_FAILED { continue; }

			let evt = match evt.data {
				Some(ref v) => match (*v).downcast_ref::<(ConnectionId, Login)>() {
					Some(v) => v.clone(),
					None => continue,
				},
				None => continue,
			};

			data.conns.send_to(evt.0, OwnedMessage::Binary(
				to_bytes(&ServerPacket::Error(Error {
					error: ErrorType::Banned
				})).unwrap()
			));
			data.conns.send_to(evt.0, OwnedMessage::Close(None));
		}
	}
}

use dispatch::SystemInfo;
use handlers::OnCloseHandler;

impl SystemInfo for LoginFailed {
	type Dependencies = OnCloseHandler;

	fn new() -> Self {
		Self::new()
	}

	fn name() -> &'static str {
		concat!(module_path!(), "::", line!())
	}
}