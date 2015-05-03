use std::sync::{Arc,Mutex};
use std::sync::mpsc::{Sender,Receiver,channel};
use std::net::UdpSocket;
use std::net::SocketAddr;
use std::io;
use std::collections::HashMap;
use std::thread;

use peer::Id;
use peer::Peer;
use kbucket::{KBuckets,K};
use proto_pinboard::Pinboard;

struct Server {
	own_id: Arc<Id>,
	kbuckets: KBuckets,

	read:  UdpSocket,
	write: UdpSocket,

	own_values: HashMap<Id, Vec<Vec<u8>>>,
	strange_values: HashMap<Id, Vec<Vec<u8>>>,

	pending_requests: Arc<Mutex<HashMap<(SocketAddr, Cookie), Sender<Message>>>>
}

const TIMEOUT_MS:usize = 3000;

#[derive(Clone,PartialEq)]
pub enum Protocol {
	Pinboard,
	Unknown
}

impl Protocol {
	fn respond_find_node(&self, sock: &UdpSocket, peer: &Peer,
	                     cookie: &Cookie, peer_list: &Vec<Peer>)
		-> io::Result<usize>
	{
		match self {
			&Protocol::Pinboard => Pinboard::respond_find_node(&sock, &peer, &cookie, &peer_list),
			&Protocol::Unknown => Err(io::Error::new(io::ErrorKind::Other, "unknown protocol")),
		}
	}

	fn respond_find_value(&self, sock: &UdpSocket, peer: &Peer, id: &Id,
	                      cookie: &Cookie, result: Result<Vec<&Vec<u8>>,Vec<Peer>>)
		-> io::Result<usize>
	{
		match self {
			&Protocol::Pinboard => Pinboard::respond_find_value(&sock, &peer, &id, &cookie, result),
			&Protocol::Unknown => Err(io::Error::new(io::ErrorKind::Other, "unknown protocol")),
		}
	}

	fn respond_pong(&self, sock: &UdpSocket, own_id: &Id, peer: &Peer, cookie: &Cookie)
		-> io::Result<usize>
	{
		match self {
			&Protocol::Pinboard => Pinboard::respond_pong(&sock, &own_id, &peer, &cookie),
			&Protocol::Unknown  => Err(io::Error::new(io::ErrorKind::Other, "unknown protocol")),
		}
	}
}

pub trait ProtocolTrait {
	fn request_ping(sock: &UdpSocket, own_id: &Id, peer: &Peer) -> io::Result<(Cookie, usize)>;
	fn respond_pong(sock: &UdpSocket, own_id: &Id, peer: &Peer, cookie: &Cookie) -> io::Result<usize>;

	fn request_find_node(sock: &UdpSocket, own_id: &Id, peer: &Peer, find_id: &Id)
		-> io::Result<(Cookie, usize)>;
	fn respond_find_node(sock: &UdpSocket, peer: &Peer,
	                     cookie: &Cookie, peer_list: &Vec<Peer>)
		-> io::Result<usize>;

	fn request_find_value(sock: &UdpSocket, own_id: &Id, peer: &Peer, find_id: &Id)
		-> io::Result<(Cookie, usize)>;
	fn respond_find_value(sock: &UdpSocket, peer: &Peer, own_id: &Id,
	                      cookie: &Cookie, result: Result<Vec<&Vec<u8>>,Vec<Peer>>)
		-> io::Result<usize>;

	fn request_store(sock: &UdpSocket, own_id: &Id, peer: &Peer, key: &Id, value: &Vec<u8>)
		-> io::Result<usize>;

	fn parse(buf: &[u8]) -> io::Result<Message>;
}

pub type Cookie = Vec<u8>;
pub type Key = Id;

#[derive(Clone)]
pub enum Message {
	Ping(Cookie, Id),
	Pong(Cookie, Id),
	FindNode(Cookie, Id, Id),
	FoundNode(Cookie, Id, Vec<Peer>),
	FindValue(Cookie, Id, Key),
	FoundValue(Cookie, Id, Vec<Vec<u8>>),
	Store(Id, Key, Vec<u8>),
	Timeout,
	Undefined,
}

impl Server {
	pub fn new(addr: &SocketAddr, own_id: Option<Id>) -> io::Result<Server> {
		let read = try!(UdpSocket::bind(addr));
		let write = try!(read.try_clone());

		let own_id = Arc::new(own_id.unwrap_or_else(|| Id::generate()));

		Ok(Server {
			own_id: own_id.clone(),
			kbuckets: KBuckets::new(own_id),
			
			read:   read,
			write:  write,

			own_values:       HashMap::new(),
			strange_values:   HashMap::new(),

			pending_requests: Arc::new(Mutex::new(HashMap::new())),
		})
	}

	pub fn ping(&self, peer: &Peer) -> io::Result<Receiver<Message>> {
		let proto = peer.protocol();
		let (cookie, _) = try!(proto.request_ping(self.write, self.own_id, &peer));

		let (tx, rx) = channel();
		let txx = tx.clone();

		let key = (peer.addr(), cookie);
		let pending_requests = self.pending_requests.lock().unwrap();
		pending_requests.insert(key.clone(), tx);

		let pending_requests = self.pending_requests.clone();

		thread::Builder::new().name("ping".to_string()).spawn(move || {
			thread::sleep_ms(TIMEOUT_MS);
			txx.send(Message::Timeout);

			let pending_requests = pending_requests.lock().unwrap();
			pending_requests.remove(key);
		}).unwrap();

		Ok(rx)
	}

	pub fn run(self) -> Arc<Mutex<Self>> {
		let sock = self.read.try_clone().unwrap();
		let this = Arc::new(Mutex::new(self));
		let that = this.clone();

		thread::Builder::new().name("DHT server".to_string()).spawn(move || {
			loop {
				let mut buf = vec![0; 16*1024];
				let (len, addr) = sock.recv_from(&mut buf).unwrap();
				buf.truncate(len);

				let mut server = this.lock().unwrap();
				server.process_message(&buf[..], &addr).unwrap();
			}
		}).unwrap();

		that
	}

	fn parse(&self, msg: &[u8]) -> io::Result<(Protocol,Message)> {
		let msg = try!(Pinboard::parse(msg));
		match msg {
			Message::Undefined => {},
			_ => return Ok((Protocol::Pinboard, msg)),
		}

		Ok((Protocol::Unknown, Message::Undefined))
	}

	pub fn process_message(&mut self, buf: &[u8], addr: &SocketAddr) -> Result<(),()> {
		let (proto, msg) = try!(self.parse(buf).map_err(|_| ()));
		match msg.clone() {
			Message::Ping(cookie, id) => {
				let peer = Peer::new(id, addr.clone(), proto.clone());
				self.kbuckets.update(&self.write, &self.own_id, &peer);

				proto.respond_pong(&self.write, &self.own_id, &peer, &cookie)
					.map(|_| ())
					.map_err(|_| ())
			},

			Message::Pong(cookie, id) => {
				// k-buckets will be updated by receiver
				// let peer = Peer::new(id, addr.clone(), proto);
				// self.kbuckets.update(&peer);
				let p = Peer::new(id, addr.clone(), proto);

				let pending_requests = self.pending_requests.lock().unwrap();
				match pending_requests.remove(&(p.addr(), cookie)) {
					Some(req) => req.send(msg).map(|_| ()).map_err(|_| ()),
					None => Err(())
				}
			},

			Message::Store(id, key, value) => {
				let peer = Peer::new(id, addr.clone(), proto);

				if !self.kbuckets.contain(&peer) {
					return Err(());
				}

				self.kbuckets.update(&peer);

				// TODO: restrict size of strange_values
				if !self.strange_values.contains_key(&key) {
					self.strange_values.insert(key.clone(), vec![]);
				}

				self.strange_values.get_mut(&key).map(|v| v.push(value));

				Ok(())
			},

			Message::FindValue(cookie, id, key) => {
				let peer = Peer::new(id, addr.clone(), proto.clone());
				self.kbuckets.update(&peer);

				let own = self.own_values.get(&key);
				let strange = self.strange_values.get(&key);

				let mut values = vec![];
				if own.is_some() {
					for v in own.unwrap() {
						values.push(v);
					}
				}
				if strange.is_some() {
					for v in strange.unwrap() {
						values.push(v);
					}
				}
				values.sort_by(|a,b| a.cmp(b));
				values.dedup();

				let response = if values.len() > 0 {
					Ok(values)
				} else {
					Err(self.kbuckets.get_nearest_peers(K, &key))
				};

				proto.respond_find_value(&self.write, &peer, &*self.own_id, &cookie, response)
					.map(|_| ())
					.map_err(|_| ())
			},

			Message::FindNode(cookie, id, find_id) => {
				let peer = Peer::new(id, addr.clone(), proto.clone());
				self.kbuckets.update(&peer);

				let peers = self.kbuckets.get_nearest_peers(K, &find_id);

				proto.respond_find_node(&self.write, &peer, &cookie, &peers)
					.map(|_| ())
					.map_err(|_| ())
			},

			Message::FoundNode(cookie, id, _)
			| Message::FoundValue(cookie, id, _) =>
			{
				let peer = Peer::new(id, addr.clone(), proto);
				self.kbuckets.update(&peer);

				match self.pending_requests.remove(&cookie) {
					Some(req) => req.send(msg).map_err(|_| ()),
					None => Err(())
				}
			},

			Message::Undefined => Err(()),
		}
	}
}
