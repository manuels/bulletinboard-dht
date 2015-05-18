use std::sync::{Arc,Mutex,MutexGuard};
use std::net::{SocketAddr};
use std::io;

use node::{Node, NodeId, NODEID_BYTELEN, xor};
use kademlia::K_PARAM;

#[cfg(test)]
use utils::ignore;

#[derive(Clone)]
pub struct KBuckets {
	own_id:  Arc<Mutex<NodeId>>,
	buckets: Vec<Arc<Mutex<Vec<Node>>>>
}

impl KBuckets {
	pub fn new(own_id: Arc<Mutex<NodeId>>) -> KBuckets {
		let buckets = (0..NODEID_BYTELEN*8)
			.map(|_| Arc::new(Mutex::new(vec![])))
			.collect();

		KBuckets {
			own_id:  own_id,
			buckets: buckets,
		}
	}

	pub fn construct_node(&mut self, addr: SocketAddr, node_id: NodeId) -> io::Result<Node> {
		let default = try!(Node::new(addr, node_id.clone()));
		let err = io::Error::new(io::ErrorKind::Other, "Hey, you stole my NodeId!");

		match self.get_bucket(&node_id) {
			None => Err(err),
			Some(ref b) => {
				let found = b.iter().find(|n| **n == default).map(|n| n.clone());
				Ok(found.unwrap_or(default))
			}
		}
	}

	fn get_bucket_idx(&self, node_id: &NodeId) -> Option<usize> {
		let own_id = {
			self.own_id.lock().unwrap()
		};

		for (i, x) in xor(&own_id, node_id).iter().enumerate() {
			for j in (0..8).rev() {
				let mask = 1<<j;

				if x & mask == mask {
					let idx = 8*(node_id.len()-1-i) + j;
					return Some(idx);
				}
			}
		}
		None
	}

	pub fn get_bucket(&self, node_id: &NodeId) -> Option<MutexGuard<Vec<Node>>> {
		self.get_bucket_idx(&node_id)
			.and_then(|i| self.buckets.get(i))
			.map(|b| b.lock().unwrap())
	}

	pub fn get_mut_bucket(&mut self, node_id: &NodeId) -> Option<MutexGuard<Vec<Node>>> {
		self.get_bucket_idx(&node_id)
			.and_then(move |i| self.buckets.get_mut(i))
			.map(|b| b.lock().unwrap())
	}

	pub fn add(&mut self, node: Node) -> Result<(), Node> {
		match self.get_mut_bucket(&node.node_id) {
			None => Ok(()), // ignore silently
			Some(ref b) if b.contains(&node) => Ok(()),
			Some(ref mut b) => {
				if b.len() < K_PARAM {
					b.push(node);
					Ok(())
				} else {
					Err(node)
				}
			}
		}
	}

	pub fn get_closest_nodes(&self, key: &NodeId, n: usize) -> Vec<Node> {
		let append = |a:Vec<Node>, b:MutexGuard<Vec<Node>>| {
			let res:Vec<Node> = a.into_iter().chain(b.clone().into_iter()).collect();
			res
		};

		let mut nodes = self.buckets.iter()
			.map(|b| b.lock().unwrap())
			.fold(vec![], append);

		let asc_dist_order = |n1:&Node, n2:&Node| n1.dist(key).cmp(&n2.dist(key));
		nodes.sort_by(asc_dist_order);
		nodes.truncate(n);

		nodes.clone()
	}
}

#[test]
fn test_get_bucket() {
	let this = vec![0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
		            0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00];
	let nearest = vec![0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
		               0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x01];
	let farest  = vec![0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,
		               0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff];

	let b = KBuckets::new(Arc::new(Mutex::new(this.clone())));
	assert_eq!(b.get_bucket_idx(&this), None);
	assert_eq!(b.get_bucket_idx(&nearest), Some(0));
	assert_eq!(b.get_bucket_idx(&farest), Some(NODEID_BYTELEN*8-1));

	assert!(b.get_bucket(&this).is_none());
	assert!(b.get_bucket(&nearest).is_some());
	assert!(b.get_bucket(&farest).is_some());
}

#[test]
fn test_get_nearest() {
	let this = vec![0x00; NODEID_BYTELEN];
	let mut b = KBuckets::new(Arc::new(Mutex::new(this.clone())));

	let mut that = this.clone();
	that[NODEID_BYTELEN-1] = 0x01;
	let n = Node::new("localhost:0", that).unwrap();
	ignore(b.add(n.clone()));

	let node_list = b.get_closest_nodes(&this, 10);
	assert_eq!(node_list, vec![n]);
}
