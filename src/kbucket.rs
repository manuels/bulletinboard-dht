use std::sync::{Arc,Mutex};
use std::thread::Builder;
use std::io::Result;

use peer::Peer;
use peer::Id;
use peer::ID_LEN;
use semaphore::Semaphore;
use server::{Server,Message};

pub const K:usize = 20;
pub const ALPHA:isize = 4;

pub struct KBuckets {
	own_id:  Arc<Id>,
	buckets: Vec<Arc<Mutex<Vec<Peer>>>>,
}

impl KBuckets {
	pub fn new(own_id: Arc<Id>) -> KBuckets {
		let buckets = (0..ID_LEN)
			.map(|_| Arc::new(Mutex::new(vec![])))
			.collect();

		KBuckets {
			own_id:  own_id,
			buckets: buckets,
		}
	}

	pub fn get_nearest_peers(&mut self, count: usize, id: &Id) -> Vec<Peer> {
		let dist = self.own_id.distance_to(id)-1;

		let mut list = vec![];
		for i in (0..dist).rev() {
			let mut bucket = self.buckets.get_mut(i).unwrap().clone();
			let mut b = bucket.lock().unwrap().clone();

			for p in b {
				list.push(p.clone());
			}

			if list.len() >= count {
				list.sort_by(|p,q| {
					let a = id.distance_to(p.id());
					let b = id.distance_to(q.id());
					a.cmp(&b)
				});

				list.truncate(count);
				return list;
			}
		}
		return list;
	}

	pub fn contain(&mut self, peer: &Peer) -> bool {
		let idx = self.own_id.distance_to(peer.id())-1;

		let mut bucket = self.buckets.get_mut(idx).unwrap().clone();

		let b = bucket.lock().unwrap();
		let pos = b.iter().position(|p| p.strictly_same_as(peer));
		
		pos.is_some()
	}

	pub fn update(&mut self, server: &Server, own_id: &Id, peer: &Peer) {
		let idx = self.own_id.distance_to(peer.id())-1;

		let mut bucket = self.buckets.get_mut(idx).unwrap().clone();

		let pos = {
			let b = bucket.lock().unwrap();
			
			b.iter().position(|p| p.strictly_same_as(peer))
		};
		let mut bucket = self.buckets.get_mut(idx).unwrap().clone();

		match pos {
			Some(idx) => {
				let mut b = bucket.lock().unwrap();
				let p = b.remove(idx);
				b.push(p);
			},
			None => {
				let sem = Semaphore::new(ALPHA);
				let inactives = Arc::new(Mutex::new(vec![]));
				let actives   = Arc::new(Mutex::new(vec![]));
				
				let mut b = bucket.lock().unwrap().clone();
				for p in b.iter().rev() {
					let guard = sem.acquire();

					// already found an inactive node? => done!
					let res = inactives.lock().unwrap();
					if res.len() > 0 {
						break
					}
					drop(res);

					let p = p.clone();
					let inactives = inactives.clone();
					let actives   = actives.clone();

					let name = format!("KBucket::update() PING {:?}", p.id());
					Builder::new().name(name).spawn(move || {
						let guard = guard;

						match server.ping(&p) {
							Ok(rx) => {
								match rx.recv() {
									Ok(Message::Pong(_,_)) => {
										let mut res = actives.lock().unwrap();
										res.push(p);
										return;
									},
									_ => {/*failed*/},
								}
							},
							_ => {/*failed*/},
						}

						/* when failed: */
						let mut res = inactives.lock().unwrap();
						res.push(p);
					}).unwrap();
				}

				let mut b = bucket.lock().unwrap();
				
				let peers = inactives.lock().unwrap();
				for p in peers.iter() {
					let idx = b.iter().position(|q| q.strictly_same_as(p)).unwrap();
					b.remove(idx);
				}

				if b.len() < K {
					b.push(peer.clone());
				}

				let peers = actives.lock().unwrap();
				for p in peers.iter() {
					let idx = b.iter().position(|q| q.strictly_same_as(p)).unwrap();
					let q = b.remove(idx);
					b.push(q);
				}
			}
		}
	}
}
