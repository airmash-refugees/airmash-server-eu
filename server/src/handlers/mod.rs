mod chat;
mod command;
mod key;
mod login;
mod onclose;
mod onopen;
mod ping;
mod pong;
mod say;
mod scoreboard;
mod signal;

pub use self::chat::ChatHandler;
pub use self::key::KeyHandler;
pub use self::login::LoginHandler;
pub use self::onclose::OnCloseHandler;
pub use self::onopen::OnOpenHandler;
pub use self::ping::PingTimerHandler;
pub use self::pong::PongHandler;
pub use self::say::SayHandler;
pub use self::scoreboard::ScoreBoardTimerHandler;
pub use self::signal::SignalHandler;

use systems;

#[deprecated]
pub type CommandHandler = (
	systems::handlers::command::Respawn,
	systems::handlers::command::Spectate,
);
