
use specs::*;

use std::any::Any;

use types::*;

use systems;
use dispatch::SystemInfo;

use component::channel::*;
use component::time::ThisFrame;
use component::flag::IsSpectating;

pub struct PlayerKilledCleanup {
	reader: Option<OnPlayerKilledReader>
}

#[derive(SystemData)]
pub struct PlayerKilledCleanupData<'a> {
	pub entities: Entities<'a>,
	pub channel: Read<'a, OnPlayerKilled>,
	pub conns: Read<'a, Connections>,
	pub thisframe: Read<'a, ThisFrame>,

	pub name: ReadStorage<'a, Name>,
	pub level: ReadStorage<'a, Level>,
	pub isspec: WriteStorage<'a, IsSpectating>,
}

impl PlayerKilledCleanup {
	pub fn new() -> Self {
		Self { reader: None }
	}
}

impl<'a> System<'a> for PlayerKilledCleanup {
	type SystemData = PlayerKilledCleanupData<'a>;

	fn setup(&mut self, res: &mut Resources) {
		Self::SystemData::setup(res);

		self.reader = Some(
			res.fetch_mut::<OnPlayerKilled>().register_reader()
		);
	}

	fn run(&mut self, mut data: Self::SystemData) {
		for evt in data.channel.read(self.reader.as_mut().unwrap()) {
			data.isspec.insert(evt.player, IsSpectating).unwrap();

			// TODO: Set a timer event to make the player respawn
		}
	}
}

impl SystemInfo for PlayerKilledCleanup {
	type Dependencies = (
		systems::missile::MissileHit
	);

	fn name() -> &'static str {
		concat!(module_path!(), "::", line!())
	}

	fn new(_: Box<Any>) -> Self {
		Self::new()
	}
}
