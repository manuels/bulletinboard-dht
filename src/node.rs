use std::sync::{Arc,Mutex};
use std::net::{SocketAddr,ToSocketAddrs};
use std::io;

use time::SteadyTime;
use rustc_serialize::{Encodable, Decodable, Encoder, Decoder};
use rustc_serialize::json;
use rustc_serialize::json::{ToJson,Json};

pub const NODEID_BYTELEN:usize = 160/8;

//pub type NodeId = [u8; NODEID_BYTELEN/8];
pub type NodeId = Vec<u8>;

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

		if !Self::is_ip_valid(&addr) {
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

	pub fn update_last_seen(&mut self) {
		let mut last_seen = self.last_seen.lock().unwrap();
		*last_seen = SteadyTime::now();
	}

	fn is_ip_valid(_: &SocketAddr) -> bool {
		/* TODO */
		true
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
