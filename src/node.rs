use std::sync::{Arc,Mutex};
use std::net::{SocketAddr,SocketAddrV4,SocketAddrV6,ToSocketAddrs};
use std::io;

use rand;
use time::SteadyTime;
use rustc_serialize::{Encodable, Decodable, Encoder, Decoder};
#[cfg(test)]
use rustc_serialize::json;
use rustc_serialize::json::{ToJson,Json};

use utils;

pub const NODEID_BYTELEN:usize = 160/8;

//pub type NodeId = [u8; NODEID_BYTELEN/8];
pub type NodeId = Vec<u8>;

macro_rules! asc_dist_order {
	($key:expr) => (|n1: &Node, n2: &Node| n1.dist(&$key).cmp(&n2.dist(&$key)))
}

macro_rules! desc_dist_order {
	($key:expr) => (|n1: &Node, n2: &Node| asc_dist_order!($key)(n1,n2).reverse())
}

#[derive(Clone, Debug)]
pub struct Node {
	pub addr:      SocketAddr,
	pub node_id:   NodeId,
	pub last_seen: Arc<Mutex<SteadyTime>>,
}

impl Node {
	pub fn new<A: ToSocketAddrs>(addr: A, node_id: NodeId) -> io::Result<Node> {
		let mut it = try!(addr.to_socket_addrs());

		let err = io::Error::new(io::ErrorKind::Other, "no valid IP address");
		let addr = try!(it.next().ok_or(err));
		let addr = utils::ip4or6(addr);

		if !Self::is_address_valid(&addr) {
			let err = io::Error::new(io::ErrorKind::Other, "no valid IP address");
			return Err(err);
		}

		let node = Node {
			addr:      addr,
			node_id:   node_id,
			last_seen: Arc::new(Mutex::new(SteadyTime::now())),
		};

		Ok(node)
	}

	pub fn generate_id() -> NodeId {
		let mut id = [0u8; NODEID_BYTELEN];
		for i in id.iter_mut() {
			*i = rand::random::<u8>();
		}
		id.to_vec()
	}

	pub fn update_last_seen(&mut self) {
		let mut last_seen = self.last_seen.lock().unwrap();
		*last_seen = SteadyTime::now();
	}

	/// TODO: replace by rust stdlib methods, as soon as they become stable
	#[cfg(not(test))]
	fn is_address_valid(addr: &SocketAddr) -> bool {
		match addr {
			&SocketAddr::V4(ref ip) => Self::is_ipv4_global(ip),
			&SocketAddr::V6(ref ip) => Self::is_ipv6_global(ip),
		}
	}

	#[cfg(test)]
	fn is_address_valid(addr: &SocketAddr) -> bool {
		true
	}

	fn is_ipv4_global(addr: &SocketAddrV4) -> bool {
		let ip = addr.ip();

		let is_private = match (ip.octets()[0], ip.octets()[1]) {
			(10, _) => true,
			(172, b) if b >= 16 && b <= 31 => true,
			(192, 168) => true,
			_ => false
		};
		let is_loopback = ip.octets()[0] == 127;

		let is_link_local = ip.octets()[0] == 169 && ip.octets()[1] == 254;

		let is_broadcast = ip.octets()[0] == 255 && ip.octets()[1] == 255 &&
			ip.octets()[2] == 255 && ip.octets()[3] == 255;

        let is_documentation = match(ip.octets()[0], ip.octets()[1], ip.octets()[2], ip.octets()[3]) {
            (192, _, 2, _) => true,
            (198, 51, 100, _) => true,
            (203, _, 113, _) => true,
            _ => false
        };


		!is_private && !is_loopback && !is_link_local &&
			!is_broadcast && !is_documentation
	}

	fn is_ipv6_global(addr: &SocketAddrV6) -> bool {
		let ip = addr.ip();

		let is_multicast = (ip.segments()[0] & 0xff00) == 0xff00;
		let is_loopback = ip.segments() == [0, 0, 0, 0, 0, 0, 0, 1];
		let is_unicast_link_local = (ip.segments()[0] & 0xffc0) == 0xfe80;
		let is_unicast_site_local = (ip.segments()[0] & 0xffc0) == 0xfec0;
		let is_unique_local = (ip.segments()[0] & 0xfe00) == 0xfc00;

		let is_unicast_global = !is_multicast
			&& !is_loopback && !is_unicast_link_local
			&& !is_unicast_site_local && !is_unique_local;

		let is_multicast_scope_global = if is_multicast {
            match ip.segments()[0] & 0x000f {
                14 => true,
                _ => false,
            }
        } else {
            false
        };

		match is_multicast_scope_global {
			true => true,
			false => is_unicast_global,
		}
	}
}

impl PartialEq for Node {
	fn eq(&self, other: &Node) -> bool {
		self.addr == other.addr && self.node_id == other.node_id
	}
}

impl ToJson for Node {
	fn to_json(&self) -> Json {
		let addr = match self.addr {
			SocketAddr::V4(addr) => {
				let ip = addr.ip().octets().iter().fold(String::new(), |s,&i| format!("{}.{}", s, i));
				format!("{}:{}", &ip[1..], addr.port())
			},
			SocketAddr::V6(addr) => {
				let ip = addr.ip().segments().iter().fold(String::new(), |s,&i| format!("{}:{}", s, i));
				format!("[{}]:{}", &ip[1..], addr.port())
			},
		};

		Json::Array(vec![addr.to_json(), self.node_id.to_json()])
    }
}

impl Encodable for Node {
	fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
		self.to_json().encode(s)
	}
}

impl Decodable for Node {
	fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
		let (addr_str, node_id): (String, Vec<u8>) = try!(Decodable::decode(d));

		if node_id.len() != NODEID_BYTELEN {
			return Err(d.error("Invalid nodeid"));
		}

		let addr = addr_str.to_socket_addrs()
				.map_err(|_| d.error("Invalid IP address"))
				.and_then(|mut it| it.next().ok_or(d.error("Invalid IP address")));
		
		Node::new(try!(addr),node_id)
			.map_err(|_| d.error("Invalid IP address"))
	}
}

pub fn xor(a: &NodeId, b: &NodeId) -> NodeId {
	let mut dist = [0u8; NODEID_BYTELEN];
	for (i, (x,y)) in a.iter().zip(b.iter()).enumerate() {
		dist[i] = x^y;
	}

	dist.to_vec()
}

impl Node {
	pub fn dist(&self, id: &NodeId) -> NodeId {
		xor(&self.node_id, id)
	}
}

#[test]
fn dist() {
	let node = Node::new("127.0.0.1:2134", vec![
		0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
		0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00]).unwrap();

	let zeros = vec![0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
		             0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00];
	let ones  = vec![0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,
		             0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff];
	let zero_ones = vec![0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
		                 0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff];
	let one_zeros = vec![0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,
		                 0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00];
	assert_eq!(node.dist(&node.node_id.clone()), zeros);
	assert_eq!(node.dist(&zeros), zeros);
	
	assert!(node.dist(&zeros) < node.dist(&ones));
	assert!(node.dist(&zeros) < node.dist(&zero_ones));
	assert!(node.dist(&zero_ones) < node.dist(&one_zeros));
	assert!(node.dist(&one_zeros) < node.dist(&ones));

	assert!([0x7f,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
		     0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00]
		     >
		    [0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
		     0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x01])
}

#[test]
fn ipv4_coding() {
	let node = Node::new("127.0.0.1:2134", vec![9;NODEID_BYTELEN]).unwrap();

	let encoded = json::encode(&node).unwrap();
	let decoded = json::decode(&encoded).unwrap();

	assert_eq!(node, decoded);
}

#[test]
fn ipv6_coding() {
	let node = Node::new("[::1]:2134", vec![1;NODEID_BYTELEN]).unwrap();

	let encoded = json::encode(&node).unwrap();
	let decoded = json::decode(&encoded).unwrap();

	assert_eq!(node, decoded);
}

#[test]
fn asc_order() {
	let id0xff = vec![0xff; NODEID_BYTELEN];
	let id0x00 = vec![0x00; NODEID_BYTELEN];

	let node0x00 = Node::new("127.0.0.1:2134", id0x00.clone()).unwrap();
	let node0xff = Node::new("127.0.0.1:2134", id0xff.clone()).unwrap();

	let mut nodes = vec![node0xff.clone(), node0x00.clone()];

	nodes.sort_by(asc_dist_order!(id0x00));
	assert_eq!(nodes, vec![node0x00.clone(), node0xff.clone()]);

	nodes.sort_by(desc_dist_order!(id0x00));
	assert_eq!(nodes, vec![node0xff, node0x00]);
}
