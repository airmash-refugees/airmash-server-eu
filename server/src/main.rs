#![allow(dead_code)]
#![feature(optin_builtin_traits)]
#![feature(trace_macros)]

// Crates with macros
#[macro_use]
extern crate log;
#[macro_use]
extern crate dimensioned;
#[macro_use]
extern crate specs_derive;
#[macro_use]
extern crate shred_derive;
#[macro_use]
extern crate lazy_static;
#[cfg_attr(feature = "serde", macro_use)]
#[cfg(feature = "serde")]
extern crate serde;

// Regular Dependencies
extern crate bit_field;
extern crate ctrlc;
extern crate fnv;
extern crate htmlescape;
extern crate phf;
extern crate rand;
extern crate rayon;
extern crate shred;
extern crate shrev;
extern crate simple_logger;
extern crate specs;
extern crate tokio;
extern crate tokio_core;
extern crate uuid;
extern crate websocket;

use websocket::futures;

// Modules
mod component;
mod consts;
mod handlers;
mod protocol;
mod server;
mod systems;
mod timeloop;
mod timers;
mod types;

use protocol as airmash_protocol;

use std::env;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use specs::{Dispatcher, DispatcherBuilder, World};
use tokio::runtime::current_thread::Runtime;

use component::time::{LastFrame, StartTime, ThisFrame};
use timeloop::timeloop;

use types::event::{ConnectionEvent, TimerEvent};

fn build_dispatcher<'a, 'b>(
	world: &mut World,
	event_recv: Receiver<ConnectionEvent>,
	timer_recv: Receiver<TimerEvent>,
	msg_recv: Receiver<(types::ConnectionId, websocket::OwnedMessage)>,
) -> Dispatcher<'a, 'b> {
	let disp = DispatcherBuilder::new()
		// Add systems here
		.with(systems::PacketHandler::new(event_recv), "packet",   &[])
		.with(systems::TimerHandler::new(timer_recv),  "timer",    &[])
		.with(systems::TimeWarn{},                     "timewarn", &[])
		.with(systems::MissileCull{},                  "missile_cull", &[]);

	let disp = systems::register(world, disp);
	let disp = systems::ctf::register(world, disp);

	disp
		// This needs to run after systems which send messages
		.with_thread_local(systems::PollComplete::new(msg_recv))

		// Build
		.build()
}

fn setup_panic_handler() {
	use std::panic;
	use std::process;

	let orig_handler = panic::take_hook();
	panic::set_hook(Box::new(move |panic_info| {
		if consts::SHUTDOWN.load(Ordering::Relaxed) {
			// This is a normal shutdown
			// no need to print to the log
			process::exit(0);
		}
		error!("A fatal error occurred within a server thread. Aborting!");
		error!("Error Info: {}", panic_info);

		orig_handler(panic_info);
		process::exit(1);
	}));
}

fn setup_interrupt_handler() {
	ctrlc::set_handler(move || {
		consts::SHUTDOWN.store(true, Ordering::Relaxed);
	}).expect("Error setting iterrupt handler");
}

fn main() {
	simple_logger::init_with_level(log::Level::Info).unwrap();
	env::set_var("RUST_BACKTRACE", "1");

	setup_panic_handler();
	setup_interrupt_handler();

	let addr = "0.0.0.0:3501";

	let mut world = World::new();

	let (event_send, event_recv) = channel::<ConnectionEvent>();
	let (timer_send, timer_recv) = channel::<TimerEvent>();
	let (msg_send, msg_recv) = channel::<(types::ConnectionId, websocket::OwnedMessage)>();

	// Add resources
	info!("Setting up resources");

	world.add_resource(types::Connections::new(msg_send));

	// Add systems
	info!("Setting up systems");

	let mut dispatcher = build_dispatcher(&mut world, event_recv, timer_recv, msg_recv);

	// Start websocket server
	info!("Starting websocket server!");
	let server_thread = thread::spawn(move || {
		server::run_acceptor(addr, event_send);
	});

	// Start gameloop
	info!("Starting gameloop!");

	// Need to run the event loop on the current
	// thread since Dispatcher doesn't implement Send
	let mut runtime = Runtime::new().unwrap();

	// Start timer loops
	let timers = thread::spawn(move || {
		tokio::run(futures::lazy(move || {
			timers::start_timer_events(timer_send);
			Ok(())
		}));
	});

	world.add_resource(StartTime(Instant::now()));
	dispatcher.setup(&mut world.res);
	world.add_resource(LastFrame(Instant::now()));

	// Add some dummmy entities so that there are no players with id 0, 1, or 2
	// this makes FFA team logic easier. The airmash client also appears to
	// make all players mimic the player with id 0
	for _ in 0..3 {
		world.create_entity().build();
	}

	// Run the gameloop at 60 Hz
	runtime.spawn(timeloop(
		move |now| {
			world.add_resource(ThisFrame(now));
			dispatcher.dispatch(&mut world.res);
			world.maintain();
			world.add_resource(LastFrame(now));
		},
		Duration::from_nanos(16666667),
	));

	runtime.run().unwrap();

	// Shut down
	info!(target: "server", "Exited gameloop, shutting down");
	server_thread.join().unwrap();
	timers.join().unwrap();

	info!(target: "server", "Shutdown completed successfully");
}