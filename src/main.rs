#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate bincode;

extern crate rand;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate crypto;
extern crate docopt;

extern crate rustc_serialize;

use bincode::{serialize, deserialize, Bounded};

#[cfg(feature="dbus")]
extern crate dbus;

#[macro_use]
mod node;
mod utils;
mod server;
mod message;
mod kademlia;
mod kbuckets;
mod closest_nodes_iter;
mod storage;

#[cfg(feature="dbus")]
mod dbus_service;

#[cfg(test)]
mod test;

use std::env;
use std::thread::{spawn,sleep};
use std::fs::File;
use std::path::{PathBuf,Path};
use std::io::{Write,Read};
use std::net::SocketAddr;
use std::time::Duration;

use docopt::Docopt;

use kademlia::Kademlia;
use node::Node;
#[cfg(feature="dbus")]
use dbus_service::dbus;

static USAGE: &'static str = "
Usage: bulletinboard [-c <path>] [-l <listen_addr>] [-j <join_addr>...]

Options:
    -h, --help                   Show this message.
    --version                    Show the version of rustc.
    -c, --config <path>          Set the path to the config file.
    -l, --listen <listen_addr>   Listen on this address.
    -j, --join <join_addr>       Bootstrap using these addresses.
";

#[derive(RustcDecodable, Debug)]
struct Args {
	flag_config:  Option<String>,
	flag_listen:  Option<String>,
	flag_join:     Vec<String>,
	flag_version: bool,
}

#[cfg(not(feature="dbus"))]
fn dbus(_: Kademlia, dbus_name: &'static str) {
}

fn load_config(cfg_path: &Path) -> Vec<SocketAddr> {
	if let Ok(mut cfg_file) = File::open(cfg_path) {
		let mut contents = String::new();
		cfg_file.read_to_string(&mut contents).unwrap_or(0);

		let nodes:Vec<Node> = deserialize(&contents[..].as_bytes()).unwrap_or(vec![]);
		nodes.iter().map(|n| n.addr).collect()
	} else {
		vec![]
	}
}

fn main() {
	env_logger::init().unwrap();

	let args: Args = Docopt::new(USAGE)
			.and_then(|d| d.parse())
			.and_then(|d| d.decode())
		.unwrap_or_else(|e| e.exit());
	debug!("{:?}", args);

	let mut default_config = env::home_dir().unwrap_or(PathBuf::from("/tmp/"));
	default_config.push(".config/bulletinboard_dht".to_string());

	let cfg_path = args.flag_config.as_ref().map_or(default_config.as_path(), |s| Path::new(s));

	let listen_addr = args.flag_listen.unwrap_or("[::]:0".to_string());

	let mut supernodes:Vec<String> = load_config(&cfg_path).iter()
		.map(|s| format!("{}", s))
		.chain(args.flag_join.into_iter())
		.collect();
	supernodes.push("[2a00:1630:66:2a9::6db4]:59056".to_string());
	supernodes.push("162.211.65.88:56789".to_string());
	supernodes.push("[2602:fff6:0001:2cf3:0000:0000:0000:1]:56790".to_string());
	supernodes.sort_by(|a,b| a.cmp(b));
	supernodes.dedup();

	let supernodes = supernodes.iter()
		.map(|s| &s[..])
		.collect();
	debug!("supernodes: {:?}", supernodes);

	let kad = Kademlia::bootstrap(&listen_addr[..], supernodes, None);

	let this = kad.clone();
	spawn(|| {
		dbus(this, "org.manuel.BulletinBoard");
	});

	loop {
		let nodes = kad.get_nodes();
		let contents = serialize(&nodes, Bounded(100*1024)).unwrap_or(Vec::new());

		if let Ok(mut cfg_file) = File::create(&cfg_path) {
			cfg_file.write(&contents[..]).unwrap_or(0);
		}

		sleep(Duration::from_secs(5*60));
	}
}
