
use specs::*;
use types::*;

use std::any::Any;

use dispatch::SystemInfo;

use component::channel::*;
use component::flag::{IsSpectating, IsPlayer};
use component::reference::PlayerRef;

use websocket::OwnedMessage;
use protocol::{to_bytes, ServerPacket};
use protocol::server::{GameSpectate, PlayerKill};

use systems::PacketHandler;

pub struct CommandHandler {
	reader: Option<OnCommandReader>
}

#[derive(SystemData)]
pub struct CommandHandlerData<'a> {
	pub channel: Read<'a, OnCommand>,
	pub conns: Read<'a, Connections>,

	pub isspec: WriteStorage<'a,IsSpectating>,
	pub spectarget: WriteStorage<'a, PlayerRef>,
	pub isplayer: ReadStorage<'a, IsPlayer>,
	pub entities: Entities<'a>,
	pub pos: ReadStorage<'a, Position>,
}

impl CommandHandler {
	pub fn new() -> Self {
		Self{ reader: None }
	}
}

impl<'a> System<'a> for CommandHandler {
	type SystemData = CommandHandlerData<'a>;

	fn setup(&mut self, res: &mut Resources) {
		Self::SystemData::setup(res);

		self.reader = Some(
			res.fetch_mut::<OnCommand>()
				.register_reader()
		);
	}

	fn run(&mut self, data: Self::SystemData) {
		let Self::SystemData {
			channel,
			conns,

			mut isspec,
			mut spectarget,
			isplayer,
			entities,
			pos
		} = data;

		for (id, packet) in channel.read(self.reader.as_mut().unwrap()) {
			if packet.com != "spectate" { continue; }

			let arg: i32 = match packet.data.parse() {
				Ok(v) => v,
				// Not a valid integer, ignore
				Err(_) => continue,
			};

			let player = match conns.associated_player(*id) {
				Some(p) => p,
				// This packet came from a connection
				// without an associated player, ignore
				None => continue,
			};

			// No valid values below -3, invalid command, ignore
			if arg < -3 { continue; }
			
			if isspec.get(player).is_none() {
				match arg {
					-3...-1 => {
						isspec.insert(player, IsSpectating).unwrap();

						let mut it = (&isplayer, &*entities)
							.join()
							.filter_map(|(_, ent)| {
								if isspec.get(ent).is_none() {
									Some(ent)
								} else {
									None
								}
							});

						if let Some(ent) = it.next() {
							let spectate = GameSpectate {
								id: ent
							};

							let killed = PlayerKill {
								id: player,
								killer: None,
								pos: *pos.get(player).unwrap()
							};

							conns.send_to_others(player, OwnedMessage::Binary(
								to_bytes(&ServerPacket::PlayerKill(killed)).unwrap()
							));

							conns.send_to_player(player, OwnedMessage::Binary(
								to_bytes(&ServerPacket::GameSpectate(spectate)).unwrap()
							));

							spectarget.insert(player, PlayerRef(ent)).unwrap();
						}
						else {
							// If there is nobody else to spectate,
							// we make the player spectate themself
							let spectate = GameSpectate {
								id: player
							};
							
							conns.send_to_player(player, OwnedMessage::Binary(
								to_bytes(&ServerPacket::GameSpectate(spectate)).unwrap()
							));

							spectarget.insert(player, PlayerRef(player)).unwrap();
						}
					},
					// Do nothing if the player didn't pass 
					// a value between -1 and -3, other values
					// only apply for players already in spec
					_ => continue,
				}
			} else {
				match arg {
					// Spectate next player
					-1 =>  {
						let current = spectarget.get(player).unwrap().0;

						// This mess gets the next player
						// including wrapping around and defaulting
						// to the current player if there is no other
						// player
						let ent = (&isplayer, &*entities)
							.join()
							.skip_while(|(_, ent)| *ent != current)
							.filter(|(_, ent)| *ent != player)
							.filter_map(|(_, ent)| {
								if isspec.get(ent).is_none() {
									return Some(ent);
								}
								None
							})
							.next()
							.unwrap_or_else(|| {
								(&isplayer, &*entities)
									.join()
									.filter(|(_, ent)| *ent != player)
									.filter_map(|(_, ent)| {
										if isspec.get(ent).is_none() {
											return Some(ent);
										}
										None
									})
									.next()
									.unwrap_or(player)
							});

						let spectate = GameSpectate {
							id: ent
						};

						conns.send_to_player(player, OwnedMessage::Binary(
							to_bytes(&ServerPacket::GameSpectate(spectate)).unwrap()
						));

						spectarget.insert(player, PlayerRef(ent)).unwrap();
					},
					// Spectate prev player
					-2 => {
						let current = spectarget.get(player).unwrap().0;
						
						let ent = (&isplayer, &*entities)
							.join()
							.take_while(|(_, ent)| *ent != current)
							.filter(|(_, ent)| *ent != player)
							.filter_map(|(_, ent)| {
								if isspec.get(ent).is_none() {
									return Some(ent);
								}
								None
							})
							.last()
							.unwrap_or_else(|| {
								(&isplayer, &*entities)
									.join()
									.filter(|(_, ent)| *ent != player)
									.filter_map(|(_, ent)| {
										if isspec.get(ent).is_none() {
											return Some(ent);
										}
										None
									})
									.last()
									.unwrap_or(player)
							});

						let spectate = GameSpectate {
							id: ent
						};

						conns.send_to_player(player, OwnedMessage::Binary(
							to_bytes(&ServerPacket::GameSpectate(spectate)).unwrap()
						));

						spectarget.insert(player, PlayerRef(ent)).unwrap();						
					},
					// Force spectate (just pick a player)
					-3 => {
						// We are already spectating a player, so 
						// we're good for now. This can be changed 
						// at a later time
						continue;
					},
					// Spectate by specific player id
					_ => {
						let ent = entities.entity(arg as u32);

						// Requested an entity that doesn't exist
						if !entities.is_alive(ent) {
							continue;
						}
						
						// The entity requested was not a player
						if isplayer.get(ent).is_none() {
							continue;
						}

						let spectate = GameSpectate {
							id: ent
						};

						conns.send_to_player(player, OwnedMessage::Binary(
							to_bytes(&ServerPacket::GameSpectate(spectate)).unwrap()
						));

						spectarget.insert(player, PlayerRef(ent)).unwrap();
					}
				}
			}
		}
	}
}

impl SystemInfo for CommandHandler {
	type Dependencies = PacketHandler;

	fn name() -> &'static str {
		concat!(module_path!(), "::", line!())
	}

	fn new(_: Box<Any>) -> Self {
		Self::new()
	}
}