use std::thread::{spawn,sleep_ms};
use std::net::{UdpSocket,SocketAddr,ToSocketAddrs};
use std::sync::{Arc,Mutex};
use std::collections::HashMap;
use std::io;

use time::{SteadyTime,Duration};
use rand;

use storage;
use server::Server;
use kbuckets::KBuckets;
use node::{Node, NodeId, NODEID_BYTELEN};
use closest_nodes_iter::ClosestNodesIter;
use message::{Message,Cookie,COOKIE_BYTELEN};
use message::{Ping,Pong, FindNode, FoundNode, FindValue, FoundValue, Store};

pub const K_PARAM: usize = 20;
pub const ALPHA_PARAM: isize = 3;
pub const timeout_ms: u32 = 2000;
pub const MAX_VALUE_LEN: usize = 2048;

#[derive(Clone)]
pub struct Kademlia {
	own_id: Arc<Mutex<NodeId>>,
	server: Server,
	kbuckets: KBuckets,
	internal_values: storage::InternalStorage,
	external_values: storage::ExternalStorage,
	TTL: Duration,
}

#[derive(PartialEq,Debug)]
enum FindJob {
	Node,
	Value,
}

impl Kademlia {
	pub fn new_supernode<A: ToSocketAddrs>(addr: A, own_id: Option<NodeId>) -> Kademlia {
		let own_id = own_id.or_else(|| Some(Self::generate_node_id()));
		Self::create(addr, own_id)
	}

	pub fn create<A: ToSocketAddrs>(addr: A, own_id: Option<NodeId>) -> Kademlia {
		let udp = UdpSocket::bind(addr).unwrap();
		let server = Server::new(udp);

		let ttl = Duration::minutes(15);
		let own_id = own_id.unwrap_or_else(|| Self::generate_node_id());
		let own_id = Arc::new(Mutex::new(own_id));

		let mut kad = Kademlia {
			own_id:   own_id.clone(),
			server:   server.clone(),
			kbuckets: KBuckets::new(own_id.clone()),
			internal_values: storage::InternalStorage::new(),
			external_values: storage::ExternalStorage::new(ttl),
			TTL:      ttl,
		};

		let mut this = kad.clone();
		spawn(move || {
			for (src, msg) in server {
				let mut this = this.clone();

				spawn(move || {
					this.handle_message(src, msg).unwrap();
				});
			}
		});

		let mut this = kad.clone();
		spawn(move || {
			// look for a random ID from time to time
			loop {
				sleep_ms(60*1000);

				let node_id = Self::generate_node_id();
				this.find_node(node_id);
			}
		});

		kad
	}

	pub fn bootstrap<A,B>(addr: A, supernodes: Vec<B>, new_id: Option<NodeId>)
		-> Kademlia
		where A: ToSocketAddrs, B: ToSocketAddrs
	{
		let mut kad = Self::create(addr, None);

		for address in supernodes.into_iter() {
			/*
			 * Let's use some random NodeId.
			 * It doesn't matter since they will be replaced automatically anyway.
			 */

			let node_id = Self::generate_node_id();
			let node = Node::new(address, node_id);

			let res = match node {
				Ok(n) => {
					let res = kad.kbuckets.add(n);
					res
				},
				Err(_) => Ok(()),
			}; // ignore this result
		}

		let mut new_id = new_id.unwrap_or_else(|| Self::generate_node_id());
		loop {
			{
				let mut own_id = kad.own_id.lock().unwrap();
				*own_id = new_id.clone();
			}

			let node_list = kad.find_node(new_id.clone());

			if !node_list.iter().any(|n|
					n.node_id == new_id &&
					n.addr != kad.server.local_addr().unwrap() //TODO: unwrap!?
				) {

				for n in node_list.into_iter() {
					kad.kbuckets.add(n);
				}

				break;
			}

			new_id = Self::generate_node_id();
		}

		kad
	}

	pub fn get(&self, key: NodeId) -> Option<Vec<Vec<u8>>> {
		self.find_value(key).ok()
	}

	pub fn put(&mut self, key: NodeId, value: Vec<u8>) -> Result<(),Vec<u8>> {
		if value.len() > MAX_VALUE_LEN {
			return Err(value);
		}

		let mut storage = self.internal_values.put(key.clone(), value.clone());

		let this = self.clone();
		let key = key.clone();
		let value = value.clone();
		spawn(move || {
			loop {
				if !this.internal_values.contains(&key, &value) {
					break
				};

				let own_id = {this.own_id.lock().unwrap().clone()};
				let req = Store {
					sender_id: own_id,
					cookie:    Self::generate_cookie(),
					key:       key.clone(),
					value:     value.clone(),
				};

				for n in this.find_node(key.clone()) {
					this.server.send_request(n.addr.clone(), &Message::Store(req.clone()));
				}

				sleep_ms((this.TTL.num_milliseconds()/2) as u32);
			}
		});
		Ok(())
	}

	pub fn remove(&mut self, key: &NodeId, value: &Vec<u8>) {
		self.internal_values.remove(key, value)
	}

	fn generate_cookie() -> Cookie {
		let cookie = Self::generate_node_id();
		assert_eq!(cookie.len(), COOKIE_BYTELEN);
		cookie
	}

	fn generate_node_id() -> NodeId {
		let mut id = [0u8; NODEID_BYTELEN];
		for i in id.iter_mut() {
			*i = rand::random::<u8>();
		}
		id.to_vec()
	}

	fn ping_or_replace_with(&mut self, replacement: Node) {
		let node_list = {
			let bucket = self.kbuckets.get_bucket(&replacement.node_id);
			if bucket.is_none() {
				return
			}

			let mut node_list:Vec<Node> = bucket.unwrap().clone();
			node_list.sort_by(|a,b| {
				let x = *a.last_seen.lock().unwrap();
				let y = *b.last_seen.lock().unwrap();
				x.cmp(&y)
			});
			node_list
		};

		let own_id = {
			self.own_id.lock().unwrap().clone()
		};
		let req = Message::Ping(Ping {
			sender_id: own_id,
			cookie:    Self::generate_cookie(),
		});

		let rx = self.server.send_many_request(node_list.into_iter(), req, timeout_ms, ALPHA_PARAM);
		
		for (node, resp) in rx.iter() {
			match resp {
				Message::Pong(_) => (),
				_ => {
					let bucket = self.kbuckets.get_mut_bucket(&replacement.node_id);
					if bucket.is_none() {
						return
					}

					let mut bucket = bucket.unwrap();
					match bucket.iter().position(|n| *n == node) {
						None => continue, // hey, where is that node gone?!
						Some(pos) => {
							bucket.remove(pos);
							bucket.push(replacement);
							return;
						}
					}
				}
			}
		}
	}

	fn update_buckets(&mut self, own_id: &NodeId, src: SocketAddr, msg: &Message)
		-> io::Result<()>
	{
		match msg {
			&Message::Timeout => (),
			_ => {
				let err_none = io::Error::new(io::ErrorKind::Other, "You don't have a NodeId!");
				let sender_id = match msg.sender_id() {
					None     => return Err(err_none),
					Some(id) => id.clone()
				};

				let err_my_id = io::Error::new(io::ErrorKind::Other, "Hey, you stole my NodeId!");
				if sender_id == *own_id {
					return Err(err_my_id);
				}

				let mut sender = try!(self.kbuckets.construct_node(src, sender_id));
				sender.update_last_seen();

				self.kbuckets.add(sender)
					.map_err(|sender| self.ping_or_replace_with(sender));
			}
		}
		Ok(())
	}

	fn handle_message(&mut self, src: SocketAddr, msg: Message)
		-> io::Result<()>
	{
		let own_id = {
			self.own_id.lock().unwrap().clone()
		};

		try!(self.update_buckets(&own_id, src, &msg));

		match msg {
			Message::Ping(ping) => {
				let pong = Pong {
					sender_id: own_id,
					cookie:    ping.cookie
				};
				self.server.send_response(src, &Message::Pong(pong));
			}
			Message::FindNode(find_node) => {
				let nodes = self.kbuckets.get_closest_nodes(&find_node.key, K_PARAM);

				let found_node = FoundNode {
					sender_id: own_id,
					cookie:    find_node.cookie,
					nodes:     nodes,
				};
				self.server.send_response(src, &Message::FoundNode(found_node));
			},
			Message::FindValue(find_value) => {
				let now = SteadyTime::now();
				
				let internal = self.internal_values.get(&find_value.key);
				let external = self.external_values.get(&find_value.key);

				let value_list:Vec<Vec<u8>> = internal.into_iter()
					.chain(external)
					.collect();

				if value_list.len() > 0 {
					let found_value = FoundValue {
						sender_id: own_id,
						cookie:    find_value.cookie,
						values:    value_list
					};
					self.server.send_response(src, &Message::FoundValue(found_value));
				} else {
					let nodes = self.kbuckets.get_closest_nodes(&find_value.key, K_PARAM);

					let found_node = FoundNode {
						sender_id: own_id,
						cookie:    find_value.cookie,
						nodes:     nodes,
					};
					self.server.send_response(src, &Message::FoundNode(found_node));
				}
			},
			Message::Store(store) => {
				if store.value.len() <= MAX_VALUE_LEN {
					self.external_values.put(store.key, store.value);
				}
			}
			Message::Timeout
			| Message::Pong(_)
			| Message::FoundNode(_)
			| Message::FoundValue(_) => (),
		};

		Ok(())
	}

	pub fn find_node(&self, key: NodeId) -> Vec<Node> {
		let res = self.find(FindJob::Node, key.clone()).unwrap_err();
		res
	}

	pub fn find_value(&self, key: NodeId) -> Result<Vec<Vec<u8>>,Vec<Node>> {
		self.find(FindJob::Value, key)
	}

	pub fn find(&self, job: FindJob, key: NodeId) -> Result<Vec<Vec<u8>>,Vec<Node>> {
		let closest = self.kbuckets.get_closest_nodes(&key, K_PARAM);

		let iter = ClosestNodesIter::new(key.clone(), K_PARAM, closest);

		let own_id = {
			self.own_id.lock().unwrap().clone()
		};

		let req = match job {
			FindJob::Node =>
				Message::FindNode(FindNode {
					cookie:    Self::generate_cookie(),
					sender_id: own_id.clone(),
					key:       key,
				}),
			FindJob::Value => {
				Message::FindValue(FindValue {
					cookie:    Self::generate_cookie(),
					sender_id: own_id.clone(),
					key:       key,
				})
			},
		};

		let rx = self.server.send_many_request(iter.clone(), req, timeout_ms, ALPHA_PARAM); //chain channels??

		let mut values = vec![];
		let mut value_nodes = K_PARAM;

		for (_, resp) in rx.iter() {
			match (resp, &job) {
				(Message::FoundNode(found_node), _) => {
					iter.add_nodes(found_node.nodes)
				},
				(Message::FoundValue(found_value), &FindJob::Value) => {
					if found_value.values.len() > 0 {
						value_nodes -= 1;
					}

					for v in found_value.values.iter() {
						values.push(v.clone());
					}
					values.dedup();

					if value_nodes == 0 {
						return Ok(values);
					}
				}
				_ => (),
			}
		}

		if values.len() > 0 {
			Ok(values)
		} else {
			Err(iter.get_closest_nodes(K_PARAM))
		}
	}
}
