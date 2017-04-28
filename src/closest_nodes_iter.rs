use std::sync::{Arc, Mutex, Condvar};
use std::sync::mpsc::Receiver;
use std::thread::spawn;

use node::{Node, NodeId};

#[cfg(test)]
use node::NODEID_BYTELEN;

#[derive(Clone)]
pub struct ClosestNodesIter {
	key: Arc<NodeId>,
	count: usize, // ask at least <count> nodes
	processed_nodes: Arc<Mutex<Vec<Node>>>,
	unprocessed_nodes: Arc<(Mutex<(Vec<Node>, usize)>, Condvar)>,
}

impl ClosestNodesIter {
	pub fn new(key: NodeId, count: usize, node_list: Vec<Node>) -> ClosestNodesIter {
		let this = ClosestNodesIter {
			key:               Arc::new(key),
			count:             count,
			processed_nodes:   Arc::new(Mutex::new(vec![])),
			unprocessed_nodes: Arc::new((Mutex::new((vec![], 0)), Condvar::new())),
		};

		this.add_nodes(node_list);
		this
	}

	#[allow(dead_code)]
	pub fn get_closest_nodes(&self, n: usize) -> Vec<Node> {
		let processed_nodes = self.processed_nodes.lock().unwrap();

		let &(ref lock, _) = &*self.unprocessed_nodes;
		let mut pair = lock.lock().unwrap();
		let &mut(ref mut unprocessed_nodes, _) = &mut *pair;

		let mut nodes = vec![];
		for n in unprocessed_nodes.iter().chain(processed_nodes.iter()) {
			nodes.push(n.clone())
		}

		let key = &self.key;
		nodes.sort_by(asc_dist_order!(key));
		nodes.truncate(n);

		nodes
	}

	pub fn add_nodes(&self, node_list: Vec<Node>) {
		// wait for locks
		let processed_nodes = self.processed_nodes.lock().unwrap();

		let &(ref lock, ref cvar) = &*self.unprocessed_nodes;
		let mut pair = lock.lock().unwrap();
		let &mut(ref mut unprocessed_nodes, _) = &mut *pair;

		// add nodes
		let iter = node_list.iter().filter(|n| !processed_nodes.contains(n));
		for n in iter {
			unprocessed_nodes.push(n.clone());
		}

		// sort nodes
		let key = &*self.key;

		unprocessed_nodes.sort_by(asc_dist_order!(key));
		unprocessed_nodes.dedup();
		unprocessed_nodes.truncate(self.count);

		// done
		cvar.notify_all();
	}

	pub fn add_node(&self, node: Node) {
		// wait for locks
		let processed_nodes = self.processed_nodes.lock().unwrap();

		let &(ref lock, ref cvar) = &*self.unprocessed_nodes;
		let mut pair = lock.lock().unwrap();
		let &mut(ref mut unprocessed_nodes, _) = &mut *pair;

		// add nodes
		if !processed_nodes.contains(&node) {
			unprocessed_nodes.push(node);
		}

		// sort nodes
		let key = &*self.key;
		unprocessed_nodes.sort_by(asc_dist_order!(key));
		unprocessed_nodes.dedup();
		unprocessed_nodes.truncate(self.count);

		// done
		cvar.notify_all();
	}

	#[allow(dead_code)]
	pub fn recv_nodes(&self, rx: Receiver<Vec<Node>>) {
		// wait for lock
		let &(ref lock, ref cvar) = &*self.unprocessed_nodes;
		let mut pair = lock.lock().unwrap();

		// increment receiver count
		let &mut (_, ref mut pending_receivers) = &mut *pair;
		*pending_receivers += 1;
		cvar.notify_all();

		let this = self.clone();
		spawn(move || {
			for addr_list in rx {
				this.add_nodes(addr_list);
			}
	
			// wait for lock
			let &(ref lock, ref cvar) = &*this.unprocessed_nodes;
			let mut pair = lock.lock().unwrap();

			// decrement receiver count
			let &mut (_, ref mut pending_receivers) = &mut *pair;
			*pending_receivers -= 1;
			cvar.notify_all();
		});
	}
}

impl Iterator for ClosestNodesIter {
	type Item = Node;

	fn next(&mut self) -> Option<Self::Item> {
		let key = &*self.key;

		loop {
			// wait for lock
			let &(ref lock, ref cvar) = &*self.unprocessed_nodes;
			let mut pair = lock.lock().unwrap();

			let mut unprocessed_nodes = pair.0.len();
			let mut pending_receivers = pair.1;

			// either we have unprocessed_nodes or we wait for pending_receviers
			while unprocessed_nodes == 0 && pending_receivers > 0 {
				pair = cvar.wait(pair).unwrap();

				unprocessed_nodes = pair.0.len();
				pending_receivers = pair.1;
			}

			let mut processed_nodes = self.processed_nodes.lock().unwrap();
			processed_nodes.sort_by(asc_dist_order!(key));
			processed_nodes.dedup();

			let &mut (ref mut unprocessed_nodes, _) = &mut *pair;
			unprocessed_nodes.sort_by(desc_dist_order!(key));
			unprocessed_nodes.dedup();

			let closest_dist = processed_nodes.get(self.count-1).map(|n| n.dist(key));

			debug!("Processed:   {}", processed_nodes.len());
			debug!("Unprocessed: {}", unprocessed_nodes.len());
			
			match unprocessed_nodes.pop() {
				None => return None,
				Some(node) => {
					processed_nodes.push(node.clone());

					if closest_dist.map(|dist| node.dist(key) >= dist).unwrap_or(false)
					{
						/*
						 * The node is not closer than the <count>th most distant
						 * node we already asked.
						 * Let's see if we will receive another node that is closer.
						 */
						debug!("looking for a closer node");
						continue
					}
					
					return Some(node)
				}
			}
		}
	}
}

#[test]
fn empty() {
	let key = [0; NODEID_BYTELEN];
	let mut iter = ClosestNodesIter::new(key, 10, vec![]);

	assert_eq!(iter.next(), None);
}

#[test]
fn clone() {
	let key = [0; NODEID_BYTELEN];

	let node = Node::new("127.0.0.1:2134", [0x00; NODEID_BYTELEN]).unwrap();
	let mut iter1 = ClosestNodesIter::new(key, 10, vec![node.clone()]);
	let mut iter2 = iter1.clone();

	assert_eq!(iter2.next(), Some(node));
	assert_eq!(iter1.next(), None);
	assert_eq!(iter2.next(), None);
}

#[test]
fn order() {
	for count in 2..4 {
		let key = [0; NODEID_BYTELEN];

		let node0xff = Node::new("127.0.0.1:2134", [0xff; NODEID_BYTELEN]).unwrap();

		let mut iter = ClosestNodesIter::new(key, count, vec![node0xff.clone()]);

		let node0x77 = Node::new("127.0.0.1:2134", [0x77; NODEID_BYTELEN]).unwrap();
		iter.clone().add_nodes(vec![node0x77.clone()]);
		assert_eq!(iter.next(), Some(node0x77));

		let node0x00 = Node::new("127.0.0.1:2134", [0x00; NODEID_BYTELEN]).unwrap();
		iter.clone().add_nodes(vec![node0x00.clone()]);
		assert_eq!(iter.next(), Some(node0x00));

		if count == 3 {
			assert_eq!(iter.next(), Some(node0xff));
		}

		assert_eq!(iter.next(), None);
	}
}
