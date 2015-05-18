extern crate rustc_serialize;
extern crate rand;
extern crate time;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate crypto;
extern crate docopt;

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

use std::thread::{spawn,sleep_ms};
use std::fs::File;
use std::io::{Write,Read};
use std::net::SocketAddr;

use docopt::Docopt;
use rustc_serialize::json;

use kademlia::Kademlia;
use node::Node;
#[cfg(feature="dbus")]
use dbus_service::dbus;

//Usage: bulletinboard [-l LISTEN_ADDR -j JOIN_ADDR...]
static USAGE: &'static str = "
Usage: bulletinboard [-l LISTENADDR -j JOINADDR...]

Options:
    -h, --help                 Show this message.
    -l, --listen LISTENADDR    Listen address.
    -j, --join JOINADDR        Join using this address.
    -c, --config CFGPATH       Path to the config file.
";

#[derive(RustcDecodable,Debug)]
struct Args {
    arg_listen_addr: Vec<String>,
    arg_join_addr:   Vec<String>,
    arg_cfgpath:    Vec<String>,
}

#[cfg(not(feature="dbus"))]
fn dbus(_: Kademlia, dbus_name: &'static str) {
}

fn load_config(cfg_path: &String) -> Vec<SocketAddr> {
	if let Ok(mut cfg_file) = File::open(cfg_path) {
		let mut contents = String::new();
		cfg_file.read_to_string(&mut contents).unwrap_or(0);

		let nodes:Vec<Node> = json::decode(&contents[..]).unwrap_or(vec![]);
		nodes.iter().map(|n| n.addr).collect()
	} else {
		vec![]
	}
}

fn main() {
	let mut args: Args = Docopt::new(USAGE)
			.and_then(|d| d.parse())
			.and_then(|d| d.decode())
		.unwrap_or_else(|e| e.exit());

	let listen_addr = args.arg_listen_addr.pop().unwrap_or("[::]:0".to_string());
	let cfg_path = args.arg_cfgpath.pop().unwrap_or("~/.config/bulletinboard_dht".to_string());

	let supernodes:Vec<String> = load_config(&cfg_path).iter()
		.map(|s| format!("{}", s))
		.chain(args.arg_join_addr.into_iter())
		.collect();

	let supernodes = supernodes.iter()
		.map(|s| &s[..])
		.collect();

	let kad = Kademlia::bootstrap(&listen_addr[..], supernodes, None);

	let this = kad.clone();
	spawn(|| {
		dbus(this, "org.manuel.BulletinBoard");
	});

	loop {
		sleep_ms(5*60*1000);
		let nodes = kad.get_nodes();
		let contents = json::encode(&nodes).unwrap_or("".to_string());

		if let Ok(mut cfg_file) = File::create(&cfg_path[..]) {
			cfg_file.write(contents.as_bytes()).unwrap_or(0);
		}
	}
}
