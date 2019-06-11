use specs::*;
use types::*;

use SystemInfo;

use component::channel::*;
use component::event::PlayerUpgrade;
use component::event::*;
use component::flag::{IsDead, IsSpectating};

use protocol::server::Error;
use protocol::ErrorType;
use protocol::UpgradeType;


use utils::{EventHandler, EventHandlerTypeProvider};

use systems::handlers::game::on_join::InitTraits;
use systems::PacketHandler;

#[derive(Default)]
pub struct Upgrade;

#[derive(SystemData)]
pub struct UpgradeData<'a> {
	pub conns: Read<'a, Connections>,
	pub channel: Write<'a, OnPlayerUpgrade>,

	pub is_spec: WriteStorage<'a, IsSpectating>,
	pub is_dead: WriteStorage<'a, IsDead>,
	pub upgrades: WriteStorage<'a, Upgrades>,
}

impl EventHandlerTypeProvider for Upgrade {
	type Event = CommandEvent;
}

impl<'a> EventHandler<'a> for Upgrade {
	type SystemData = UpgradeData<'a>;

	fn on_event(&mut self, evt: &CommandEvent, data: &mut UpgradeData<'a>) {
		let &(conn, ref packet) = evt;

		let player = match data.conns.associated_player(conn) {
			Some(p) => p,
			None => return,
		};

		if packet.com != "upgrade" {
			return;
		}

		let ty = match parse_upgrade_data(&packet.data) {
			Ok(ty) => ty,
			Err(_) => return,
		};

		if data.is_dead.get(player).is_some() {
			println!("upgrade prohibited, cant apply while dead");
			return;
		}

		if data.is_spec.get(player).is_some() {
			println!("upgrade prohibited, cant apply while spectating");
			return;
		}

		let mut upgrades = *(
			data.upgrades
			.get(player)
			.unwrap()
			//.unwrap_or_else(Upgrades::default());
		);

		upgrades.unused = 5;
		if upgrades.unused == 0 {
			println!("upgrade prohibited, user has none");
			data.conns.send_to(
				conn,
				Error {
					error: ErrorType::NotEnoughUpgrades,
				},
			);
			return;
		} else {
			let field = match ty {
				UpgradeType::Speed => &mut upgrades.speed,
				UpgradeType::Defense => &mut upgrades.defense,
				UpgradeType::Energy => &mut upgrades.energy,
				UpgradeType::Missile => &mut upgrades.missile,
				_ => {
					println!("this code cannot be reached.");
					return;
				}
			};

			if *field == 5 {
				println!("upgrade prohibited, type {:?} maxed out", ty);
				return;
			}

			upgrades.unused -= 1;
			*field += 1;
			println!("upgraded {:?}, {:?}, new value {}", player, ty, field);
		}

		data.upgrades.insert(player, upgrades).unwrap();

		data.channel.single_write(
			PlayerUpgrade {
				player: player,
				ty: ty
			}
		);

		data.conns.send_to(
			conn,
			protocol::server::PlayerUpgrade {
				speed: upgrades.speed,
				defense: upgrades.defense,
				energy: upgrades.energy,
				missile: upgrades.missile,
				upgrades: upgrades.unused,
				ty: ty,
			}
		);
	}
}

impl SystemInfo for Upgrade {
	type Dependencies = (PacketHandler, InitTraits);

	fn name() -> &'static str {
		concat!(module_path!(), "::", line!())
	}

	fn new() -> Self {
		Self::default()
	}
}


fn parse_upgrade_data(s: &str) -> Result<UpgradeType, ()> {
	match s.parse().unwrap_or_default() {
		1 => Ok(UpgradeType::Speed),
		2 => Ok(UpgradeType::Defense),
		3 => Ok(UpgradeType::Energy),
		4 => Ok(UpgradeType::Missile),
		_ => Err(())
	}
}
