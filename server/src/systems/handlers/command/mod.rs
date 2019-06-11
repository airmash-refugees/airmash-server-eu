mod register;

mod flag;
mod respawn;
mod spectate;
mod upgrade;

pub use self::flag::Flag;
pub use self::respawn::Respawn;
pub use self::spectate::Spectate;
pub use self::upgrade::Upgrade;

pub use self::register::register;

pub type AllCommandHandlers = (Flag, Respawn, Spectate);
