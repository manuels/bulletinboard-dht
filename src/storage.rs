use std::sync::{Arc,Mutex};
use std::collections::HashMap;
use time::{SteadyTime,Duration};
use std::net::SocketAddr;

use node::NodeId;

#[allow(non_snake_case)]
#[derive(Clone)]
pub struct ExternalStorage {
	storage: Arc<Mutex<HashMap<NodeId, Vec<(Vec<u8>, (SocketAddr, NodeId), SteadyTime)>>>>,
	TTL:     Duration,
}

impl ExternalStorage {
	pub fn new(ttl: Duration) -> ExternalStorage {
		ExternalStorage {
			storage: Arc::new(Mutex::new(HashMap::new())),
			TTL: ttl,
		}
	}

	pub fn put(&mut self, key: NodeId, sender: (SocketAddr, NodeId), value: Vec<u8>) {
		self.cleanup();

		let mut storage = self.storage.lock().unwrap();
		
		let mut s = storage.remove(&key).unwrap_or(vec![]);
		s.iter()
			.position(|&(ref v, ref s, _)| *v == value || *s == sender)
			.map(|pos| s.remove(pos));
		
		let now = SteadyTime::now();
		s.push((value, sender, now));

		storage.insert(key, s);
	}

	pub fn get(&mut self, key: &NodeId) -> Vec<Vec<u8>> {
		self.cleanup();

		let storage = self.storage.lock().unwrap();

		match storage.get(key) {
			None => vec![],
			Some(vec) => vec.clone().into_iter()
				.map(|(v,_,_)| v).collect()
		}
	}

	fn cleanup(&mut self) {
		let now = SteadyTime::now();
		let mut storage = self.storage.lock().unwrap();

		for (_, values) in storage.iter_mut() {
			*values = (*values).clone().into_iter()
				.filter(|&(_, _, ref ttl)| (*ttl)+self.TTL > now)
				.collect();
		}
	}
}
