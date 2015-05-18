use std::sync::{Arc,Mutex};
use std::collections::HashMap;
use time::{SteadyTime,Duration};

use node::NodeId;

#[derive(Clone)]
pub struct InternalStorage {
	storage: Arc<Mutex<HashMap<NodeId, Vec<Vec<u8>>>>>,
}

#[allow(non_snake_case)]
#[derive(Clone)]
pub struct ExternalStorage {
	storage: Arc<Mutex<HashMap<NodeId, Vec<(Vec<u8>, SteadyTime)>>>>,
	TTL:     Duration,
}

impl InternalStorage {
	pub fn new() -> InternalStorage {
		InternalStorage {
			storage: Arc::new(Mutex::new(HashMap::new())),
		}
	}

	pub fn put(&mut self, key: NodeId, value: Vec<u8>) {
		let mut storage = self.storage.lock().unwrap();
		let mut s = storage.remove(&key).unwrap_or(vec![]);

		s.push(value);
		s.dedup();
		storage.insert(key, s);
	}

	pub fn contains(&self, key: &NodeId, value: &Vec<u8>) -> bool {
		let storage = self.storage.lock().unwrap();

		match storage.get(key) {
			None => false,
			Some(list) => list.contains(value)
		}
	}

	pub fn get(&self, key: &NodeId) -> Vec<Vec<u8>> {
		let storage = self.storage.lock().unwrap();

		storage.get(key)
			.map(|v| v.clone())
			.unwrap_or(vec![])
	}

	pub fn remove(&mut self, key: &NodeId, value: &Vec<u8>) {
		let mut storage = self.storage.lock().unwrap();
		match storage.get_mut(key) {
			None => (),
			Some(values) => {
				match values.iter().position(|v| v == value) {
					None => (),
					Some(pos) => {
						values.remove(pos);
					}
				}
			}
		}
	}

	pub fn remove_key(&mut self, key: &NodeId) {
		let mut storage = self.storage.lock().unwrap();
		storage.remove(key);
	}
}

impl ExternalStorage {
	pub fn new(ttl: Duration) -> ExternalStorage {
		ExternalStorage {
			storage: Arc::new(Mutex::new(HashMap::new())),
			TTL: ttl,
		}
	}

	pub fn put(&mut self, key: NodeId, value: Vec<u8>) {
		self.cleanup();

		let mut storage = self.storage.lock().unwrap();
		
		let mut s = storage.remove(&key).unwrap_or(vec![]);
		s.iter()
			.position(|&(ref v,_)| *v == value)
			.map(|pos| s.remove(pos));
		
		let now = SteadyTime::now();
		s.push((value, now));

		storage.insert(key, s);
	}

	pub fn get(&mut self, key: &NodeId) -> Vec<Vec<u8>> {
		self.cleanup();

		let storage = self.storage.lock().unwrap();

		match storage.get(key) {
			None => vec![],
			Some(vec) => vec.clone().into_iter().map(|(v,_)| v).collect()
		}
	}

	fn cleanup(&mut self) {
		let now = SteadyTime::now();
		let mut storage = self.storage.lock().unwrap();

		for (_, values) in storage.iter_mut() {
			*values = (*values).clone().into_iter()
				.filter(|&(_, ref ttl)| (*ttl)+self.TTL > now)
				.collect();
		}
	}
}
