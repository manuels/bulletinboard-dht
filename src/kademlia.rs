use std::io;
use std::sync::mpsc;
use std::thread::{spawn,sleep};
use std::net::{UdpSocket,SocketAddr,ToSocketAddrs};
use std::sync::{Arc,Mutex,RwLock};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

use futures::Future;
use futures::Stream;
use tokio_core::reactor::Handle;
use tokio_core::reactor::Interval;
use futures_timer::Sleep;

use storage;
use server::Server;
use kbuckets::KBuckets;
use node::{Node, NodeId};
use closest_nodes_iter::ClosestNodesIter;
use message::{Message,Value,Cookie,COOKIE_BYTELEN};
use message::{Ping,Pong, FindNode, FoundNode, FindValue, FoundValue, Store};
use utils::ignore;
use message::enc_id;

pub const K_PARAM: usize = 20;
pub const ALPHA_PARAM: isize = 3;
pub const TIMEOUT_MS: u32 = 2000;
pub const MAX_VALUE_LEN: usize = 2048;

#[derive(Clone)]
pub struct Kademlia {
	own_id: Arc<Mutex<NodeId>>,
	stored_values: Arc<RwLock<HashMap<NodeId, (u64, Vec<u8>)>>>,
	server: Server,
	kbuckets: KBuckets,
	external_values: storage::ExternalStorage,
	listeners: storage::ExternalStorage,
	ttl: Duration,
}

impl Kademlia {
	#[allow(dead_code)]
	pub fn new_supernode<A: ToSocketAddrs>(handle: Handle, addr: A, own_id: Option<NodeId>) -> Kademlia {
		let own_id = own_id.or_else(|| Some(Node::generate_id()));
		Self::create(handle, addr, own_id)
	}

	pub fn create<A: ToSocketAddrs>(handle: Handle, addr: A, own_id: Option<NodeId>) -> Kademlia {
		let udp = UdpSocket::bind(addr).unwrap();
		let server = Server::new(handle, udp);

		let ttl = Duration::from_secs(15*60);
		let own_id = own_id.unwrap_or_else(|| Node::generate_id());
		let own_id = Arc::new(Mutex::new(own_id));

		let kad = Kademlia {
			own_id:          own_id.clone(),
			server:          server.clone(),
			stored_values:   Arc::new(RwLock::new(HashMap::new())),
			kbuckets:        KBuckets::new(own_id),
			external_values: storage::ExternalStorage::new(ttl),
			listeners:       storage::ExternalStorage::new(ttl),
			ttl:             ttl,
		};

		let this = kad.clone();
		let handle = this.server.handle.clone();
		handle.spawn_fn(move || {
			for (src, msg) in server {
				let mut this = this.clone();

				handle.spawn_fn(move || {
					ignore(this.handle_message(src, msg));
					Ok(())
				});
			}
			Ok(())
		});

		let this = kad.clone();
		let handle = this.server.handle.clone();
		handle.spawn(Interval::new(Duration::from_secs(60), &handle).unwrap().for_each(move |_| {
			let node_id = Node::generate_id();
			this.find_node(node_id);
			Ok(()) as Result<(), io::Error>
		}).map_err(|_| ()));

		let mut this = kad.clone();
		handle.spawn(Interval::new(Duration::from_secs(5*60), &handle).unwrap().for_each(move |_| {
			// publish stored values again and again
			let stored_values = this.stored_values.clone();
			let mut store = stored_values.write().unwrap();

			for (key, t) in store.iter_mut() {
				let (ref mut lifetime, ref value) = *t;
				*lifetime = lifetime.saturating_sub(5 * 60);

				if *lifetime > 0 {
					this.put(*key, value.clone()).unwrap();
				}
			}

			Ok(()) as Result<(), io::Error>
		}).map_err(|_| ()));

		kad
	}

	pub fn bootstrap<A,B>(handle: Handle, addr: A, supernodes: Vec<B>, new_id: Option<NodeId>)
		-> Kademlia
		where A: ToSocketAddrs, B: ToSocketAddrs
	{
		let mut kad = Self::create(handle, addr, None);

		for address in supernodes.into_iter() {
			/*
			 * Let's use some random NodeId.
			 * It doesn't matter since they will be replaced automatically anyway.
			 */

			let node_id = Node::generate_id();
			let node = Node::new(address, node_id);

			ignore(node.map(|n| kad.kbuckets.add(n)));
		}

		let mut new_id = new_id.unwrap_or_else(|| Node::generate_id());
		loop {
			kad.set_own_id(new_id);

			let node_list = kad.find_node(new_id);

			if !node_list.iter().any(|n|
					n.node_id == new_id &&
					n.addr != kad.server.local_addr().unwrap() //TODO: unwrap!?
				) {

				for n in node_list.into_iter() {
					ignore(kad.kbuckets.add(n));
				}

				break;
			}

			new_id = Node::generate_id();
		}

		kad
	}

	pub fn get_nodes(&self) -> Vec<Node> {
		self.kbuckets.get_nodes()
	}

	pub fn get(&self, key: NodeId) -> Vec<Vec<u8>> {
		debug!("Finding {}...", enc_id(&key));
        let values:Vec<Vec<u8>> = self.find_value(key).iter().collect();
		if values.len() > 0 {
			info!("Found {:?} values for {}", values.len(), enc_id(&key));
		} else {
			warn!("Found NO values for {}", enc_id(&key));
		}
		values
	}

	pub fn get_own_id(&self) -> NodeId {
		self.own_id.lock().unwrap().clone()
	}

	fn set_own_id(&self, new_id: NodeId) {
		let mut own_id = self.own_id.lock().unwrap();
		*own_id = new_id;
	}

    /// Just store a value once
    #[async]
	pub fn put(&mut self, key: NodeId, value: Vec<u8>) -> Result<(),Vec<u8>> {
		if value.len() > MAX_VALUE_LEN {
			return Err(value);
		}

		Ok(self.publish(key, value))
	}

    /// Store a value permanently for `lifetime`
    #[async]
	pub fn store(&mut self, key: NodeId, value: Vec<u8>, lifetime: u64) -> Result<(),Vec<u8>> {
		await!(self.put(key, value));

	    spawn(iter.take(lifetime/timeout).map(|| {
    		self.put(key, value);
        }));
	}

    #[async]
	fn publish(&self, key: NodeId, value: Vec<u8>) -> Result<(),()> {
		let msg = Message::Store(Store {
			sender_id: self.get_own_id(),
			cookie:    Self::generate_cookie(),
			key:       key,
			value:     Value::new(value),
		});

		let nodes = await!(self.find_node(key));
		let nodes_len = nodes.len();

		for n in nodes {
			self.server.hit_and_run(n.addr, &msg);
		}

		if nodes_len > 0 {
			info!("Published {} on {:?} nodes.", enc_id(&key), nodes_len);
		} else {
			warn!("Could not find any nodes to publish {}!", enc_id(&key));
		}
	}

	fn generate_cookie() -> Cookie {
		let cookie = Node::generate_id();
		assert_eq!(cookie.len(), COOKIE_BYTELEN);
		cookie
	}

	fn ping_or_replace_with(&mut self, replacement: Node) {
		let node_list = {
			let bucket = self.kbuckets.get_bucket(&replacement.node_id);

			let mut node_list:Vec<Node> = bucket.map(|b| b.clone()).unwrap_or(vec![]);
			node_list.sort_by(|a,b| {
				let x = *a.last_seen.lock().unwrap();
				let y = *b.last_seen.lock().unwrap();
				x.cmp(&y)
			});
			node_list
		};

		let req = Message::Ping(Ping {
			sender_id: self.get_own_id(),
			cookie:    Self::generate_cookie(),
		});

		let rx = self.server.send_many_request(node_list.into_iter(), req, TIMEOUT_MS, ALPHA_PARAM);
		
		for (node, resp) in rx {
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
					Some(id) => id
				};

				let err_my_id = io::Error::new(io::ErrorKind::Other, "Hey, you stole my NodeId!");
				if sender_id == *own_id {
					return Err(err_my_id);
				}

				let mut sender = try!(self.kbuckets.construct_node(src, sender_id));
				sender.update_last_seen();

				ignore(self.kbuckets.add(sender)
					.map_err(|sender| self.ping_or_replace_with(sender)));
			}
		}

//        info!("Approximately {} peers in the network.", self.kbuckets.estimate_peers_in_network());

		Ok(())
	}

	fn handle_message(&mut self, src: SocketAddr, msg: Message)
		-> io::Result<()>
	{
		let own_id = self.get_own_id();

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
				let node_list = self.kbuckets.get_closest_nodes(&find_node.key, K_PARAM);
				let count = node_list.len();

				for node in node_list.into_iter() {
					let found_node = FoundNode {
						sender_id:  own_id,
						cookie:     find_node.cookie,
						node_count: count,
						node:       node,
					};
					self.server.send_response(src, &Message::FoundNode(found_node));
				}
			},
			Message::FindValue(find_value) => {
				let value_list = self.external_values.get(&find_value.key);

				if value_list.len() > 0 {
					let count = value_list.len();

					for (_, value) in value_list.into_iter() {
						let found_value = FoundValue {
							sender_id:   own_id,
							cookie:      find_value.cookie,
							value_count: count,
							value:       Value::new(value),
						};
						self.server.send_response(src, &Message::FoundValue(found_value));
					}
				} else {
					let node_list = self.kbuckets.get_closest_nodes(&find_value.key, K_PARAM);
					let count = node_list.len();

					for node in node_list.into_iter() {
						let found_node = FoundNode {
							sender_id:  own_id,
							cookie:     find_value.cookie,
							node_count: count,
							node:       node,
						};
						self.server.send_response(src, &Message::FoundNode(found_node));
					}
				}
			},
			Message::Store(store) => {
				if store.value.len() <= MAX_VALUE_LEN {
					let sender = (src, store.sender_id);
					self.external_values.put(store.key, sender, (*store.value).clone());

					for ((dst, _), cookie_vec) in self.listeners.get(&store.key) {
					    let mut cookie = [0; COOKIE_BYTELEN];
					    cookie.copy_from_slice(&cookie_vec);

                        let found_value = FoundValue {
							sender_id:   own_id,
							cookie:      cookie,
							value_count: 1,
							value:       Value::new((*store.value).clone()),
                        };
                        self.server.send_response(dst, &Message::FoundValue(found_value));
					}
				}
			},
			Message::Listen(listen) => {
				let sender = (src, listen.sender_id);
                self.listeners.put(listen.key, sender, listen.cookie.to_vec())
			},
			Message::Timeout
			| Message::Pong(_)
			| Message::FoundNode(_)
			| Message::FoundValue(_) => (),
		};

		Ok(())
	}

	fn find_value(&self, key: NodeId) -> impl Stream<Vec<u8>> {
        let own_id = self.get_own_id();
		let closest = self.kbuckets.get_nodes();
	    debug!("FindValue: {:?} initial nodes", closest.len());

	    let iter = ClosestNodesIter::new(key, K_PARAM, closest);

	    let req = Message::FindValue(FindValue {
		    cookie:    Self::generate_cookie(),
		    sender_id: self.get_own_id(),
		    key:       key,
	    });
	    let rx = self.server.send_many_request(iter.clone(), req, TIMEOUT_MS, ALPHA_PARAM); //chain channels??

        let (result_tx, result_rx) = mpsc::channel(2048);

	    let mut values = HashSet::new();
	    let mut value_nodes = HashSet::new();
        let timeout = Sleep::new(Duration::from_millis(TIMEOUT_MS));

        #[async]
        for resp in rx.select2(timeout) {
            if let Either::A((_, resp), timeout) = res? {
			    match resp {
				    Message::FoundNode(found_node) => {
					    let node = found_node.node;

					    if node.node_id != own_id {
						    iter.add_node(node);
					    }
				    },
				    Message::FoundValue(FoundValue { sender_id: id, value: Value { data: v }, .. }) => {
					    if !values.contains(&v) {
				            //result_tx.send(v.clone()).unwrap();
				            yield v.clone();
				            values.insert(v);
					    }

                        value_nodes.insert(id);
					    if value_nodes.len() == K_PARAM {
						    return;
					    }
				    },
			    }

                timeout.reset();
            }

        }
        handle.spawn(future);

        result_rx
	}

    #[async]
	fn find_node(&self, key: NodeId) -> Result<Vec<Node>> {
		let closest = self.kbuckets.get_nodes();

		debug!("FindNode: {:?} initial nodes", closest.len());
		let iter = ClosestNodesIter::new(key, K_PARAM, closest);

		let req = Message::FindNode(FindNode {
			cookie:    Self::generate_cookie(),
			sender_id: self.get_own_id(),
			key:       key,
		});
		let rx = self.server.send_many_request(iter.clone(), req, TIMEOUT_MS, ALPHA_PARAM); //chain channels??
        let timeout = Sleep::new(Duration::from_millis(4*TIMEOUT_MS));

        rx.select(timeout)
          .inspect(|n| if node.node_id != own_id { iter.add_node(n) })
          .take_while(|| is_not_timeout)
          .fold(vec![], |nodes, n| Ok(nodes))

	    let mut nodes_online = vec![];
	    let mut failed = 0;
	    while failed < TIMEOUT_MS/250 {
		    for (sender, resp) in rx.iter() {
			    debug!("resp={:?}", resp);
			    failed = 0;

                if let Message::FoundNode(found_node) = resp {
				    nodes_online.push(sender);
				    nodes_online.sort_by(asc_dist_order!(key));
				    nodes_online.dedup();

				    let own_id = self.get_own_id();
				    let node = found_node.node;

				    if node.node_id != own_id {
					    iter.add_node(node);
				    }
			    };
		    }

		    sleep(Duration::from_millis(250));
		    failed += 1;
	    }

	    nodes_online.truncate(K_PARAM);
	    nodes_online
	}
}
 
