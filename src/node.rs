use std::io;
use std::fmt;
use std::time::Instant;
use std::sync::{Arc,Mutex};
use std::net::{SocketAddr,ToSocketAddrs};

#[cfg(not(test))]
use std::net::{SocketAddrV4,SocketAddrV6};

use rand;
use utils;
use message::enc_id;

pub const NODEID_BYTELEN:usize = 160/8;

pub type NodeId = [u8; NODEID_BYTELEN];

macro_rules! asc_dist_order {
	($key:expr) => (|n1: &Node, n2: &Node| n1.dist(&$key).cmp(&n2.dist(&$key)))
}

macro_rules! desc_dist_order {
	($key:expr) => (|n1: &Node, n2: &Node| asc_dist_order!($key)(n1,n2).reverse())
}

fn now_mutex() -> Arc<Mutex<Instant>> {
	Arc::new(Mutex::new(Instant::now()))
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Node {
	pub addr:      SocketAddr,
	pub node_id:   NodeId,
	#[serde(skip_serializing)]
	#[serde(skip_deserializing,default="now_mutex")]
	pub last_seen: Arc<Mutex<Instant>>,
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
			last_seen: Arc::new(Mutex::new(Instant::now())),
		};

		Ok(node)
	}

	pub fn generate_id() -> NodeId {
		let mut id = [0u8; NODEID_BYTELEN];
		for i in id.iter_mut() {
			*i = rand::random::<u8>();
		}
		id
	}

	pub fn update_last_seen(&mut self) {
		let mut last_seen = self.last_seen.lock().unwrap();
		*last_seen = Instant::now();
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
	fn is_address_valid(_addr: &SocketAddr) -> bool {
		true
	}

	#[cfg(not(test))]
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

	#[cfg(not(test))]
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

pub fn xor(a: &NodeId, b: &NodeId) -> NodeId {
	let mut dist = [0u8; NODEID_BYTELEN];
	for (i, (x,y)) in a.iter().zip(b.iter()).enumerate() {
		dist[i] = x^y;
	}

	dist
}

impl Node {
	pub fn dist(&self, id: &NodeId) -> NodeId {
		xor(&self.node_id, id)
	}
}

impl fmt::Debug for Node {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
	    let secs = self.last_seen.lock().unwrap().elapsed().as_secs() as f64;
		write!(f, "Node {{ {}, id={}, last_seen={:.*}min ago }}",
			self.addr, enc_id(&self.node_id), 2, secs/60.0)
	}
}

#[test]
fn dist() {
	let node = Node::new("127.0.0.1:2134", [
		0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
		0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00]).unwrap();

	let zeros = [0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
		             0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00];
	let ones  = [0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,
		             0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff];
	let zero_ones = [0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
		                 0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff];
	let one_zeros = [0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,
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
fn asc_order() {
	let id0xff = [0xff; NODEID_BYTELEN];
	let id0x00 = [0x00; NODEID_BYTELEN];

	let node0x00 = Node::new("127.0.0.1:2134", id0x00.clone()).unwrap();
	let node0xff = Node::new("127.0.0.1:2134", id0xff.clone()).unwrap();

	let mut nodes = vec![node0xff.clone(), node0x00.clone()];

	nodes.sort_by(asc_dist_order!(id0x00));
	assert_eq!(nodes, vec![node0x00.clone(), node0xff.clone()]);

	nodes.sort_by(desc_dist_order!(id0x00));
	assert_eq!(nodes, vec![node0xff, node0x00]);
}
