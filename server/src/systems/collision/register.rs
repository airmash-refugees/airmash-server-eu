
use specs::*;

use systems::collision::bounce::BounceSystem;
use systems::collision::plane::PlaneCollisionSystem;
use systems::collision::missile::MissileTerrainCollisionSystem;

pub fn register<'a, 'b>(
	_: &mut World, 
	disp: DispatcherBuilder<'a, 'b>
) -> DispatcherBuilder<'a, 'b> {
	disp
		.with(
			PlaneCollisionSystem::new(),
			"collision_plane-terrain",
			&["position_update"]
		)
		.with(
			MissileTerrainCollisionSystem::new(),
			"collision_missile-terrain",
			// I don't think this is right
			// TODO: Determine actual system
			&["position_update"]
		)
		.with(
			BounceSystem::new(),
			"collision_bounce",
			&["collision_plane-terrain"]
		)
}