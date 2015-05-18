extern crate rustc_serialize;
extern crate rand;
extern crate time;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate crypto;

#[cfg(feature="dbus")]
extern crate dbus;

mod utils;
mod server;
mod message;
mod kademlia;
mod kbuckets;
mod node;
mod closest_nodes_iter;
mod storage;

#[cfg(feature="dbus")]
mod dbus_service;

#[cfg(test)]
mod test;

use kademlia::Kademlia;

#[cfg(feature="dbus")]
use dbus_service::dbus;

#[cfg(not(feature="dbus"))]
fn dbus(_: Kademlia, dbus_name: &'static str) {
}

fn main() {
	let supernodes = vec!["[::1]:23121"];
	let kad = Kademlia::bootstrap("[::]:0", supernodes, None);

	dbus(kad, "org.manuel.BulletinBoard");
}
