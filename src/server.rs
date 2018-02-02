use std::time::Duration;
use std::thread::{spawn,sleep};
use std::sync::mpsc::{Sender,Receiver,channel};
use std::sync::{Arc,Mutex};
use std::str;
use std::io;
use std::net::{SocketAddr};
use std::collections::HashMap;

use bincode::{serialize, deserialize, Bounded};

use futures::prelude::*;
use futures::Future;
use tokio_core::reactor::Handle;
use tokio_core::reactor::Timeout;
use tokio_core::net::UdpSocket;
use tokio_core::net::UdpFramed;

use utils::ignore;
use utils;
use utils::semaphore::Semaphore;
use message::{Message, Cookie};
use node::Node;

pub struct Server {
	handle: Handle,
	pub local_addr: SocketAddr,
	sink: SplitSink<UdpFramed<Codec>>,
	stream:  SplitStream<UdpFramed<Codec>>,
	pending_requests: Rc<RefCell<HashMap<(SocketAddr, Cookie), Sender<Message>>>>
}

// TODO: cleanup 'pending_requests' from time to time!

impl Server {
	pub fn new(handle: Handle, sock: UdpSocket) -> Result<Server> {
		info!("Listening on {:?}", sock.local_addr());
		let local_addr = sock.local_addr()?;
		let sink, stream = sock.framed(Codec).split();
		Server {
			handle,
			local_addr,
			sink,
			stream
			pending_requests: Rc::new(RefCell::new(HashMap::new())),
		}
	}

	/// just send a message and don't care about the reponse
	pub fn hit_and_run(&self, addr: SocketAddr, req: &Message) {
		self.send(addr, req);
	}

	#[allow(dead_code)]
	pub fn send_request(&self, addr: SocketAddr, req: &Message) -> Message
	{
		let rx = self.send(addr, req);
		rx.recv().unwrap()
	}

	fn send(&self, addr: SocketAddr, req: &Message) -> Receiver<Message>
	{
		debug!("Sending {:?} to {:?}", req, addr);
		let (tx, rx) = channel();

		{
			let mut pending = self.pending_requests.lock().unwrap();
			let key = (addr, *req.cookie().unwrap());
			(*pending).insert(key, tx);
		}

		let buf = serialize(&req, Bounded(2048)).unwrap();
		self.sock.send_to(&buf[..], addr).unwrap();

		rx
	}

	pub fn send_response(&self, addr: SocketAddr, resp: &Message)
	{
		let buf = serialize(&resp, Bounded(2048)).unwrap();
		self.sock.send_to(&buf[..], addr).unwrap();
	}

	pub fn send_request_ms(&self, addr: &SocketAddr, req: &Message, timeout: u32)
		-> Receiver<Message>
	{
		let (tx, rx) = channel();

		{
			let mut pending = self.pending_requests.lock().unwrap();
			let key = (*addr, *req.cookie().unwrap());
			(*pending).insert(key, tx.clone());
		}

		debug!("Sending {:?} to {:?}", req, addr);
		let buf = serialize(&req, Bounded(2048)).unwrap();
		ignore(self.sock.send_to(&buf[..], addr));

		let handle = self.handle.clone();
		handle.spawn_fn(move || {
			Timeout::new(Duration::from_millis(timeout as u64), &handle).unwrap().then(move |_| {
				match tx.send(Message::Timeout) {
					Ok(_) => Ok(()),
					Err(_) => Ok(()),
				}
			})
		});

		rx
	}

	/// returns an Channel you can use as an Iterator of type [(addr_index, Message), ...]
	///
	/// just consume it until you got a reponse that satisfies your requirements
	/// (You probably do not want to call iter.collect(): it will ask ALL nodes!)
	pub fn send_many_request<I>(&self, iter: I, req: Message,
	                    timeout: u32, concurrency: isize)
		-> Receiver<(Node, Message)>
			where I: 'static + Iterator<Item=Node> + Send
	{
		let is_rx_dead = Arc::new(Mutex::new(false));
		let (tx, rx) = channel();

		let this = self.clone();
		self.handle.spawn_fn(move || {
			let sem = Arc::new(Semaphore::new(concurrency));

			for node in iter.take_while(|_| *(is_rx_dead.lock().unwrap()) == false) {
				let is_rx_dead = is_rx_dead.clone();
				let req = req.clone();
				let node = node.clone();
				let this = this.clone();
				let sem = sem.clone();
				let tx = tx.clone();

				// acquire BEFORE we spawn!
				sem.acquire();

				this.handle.spawn_fn(move || {
					let rx = this.send_request_ms(&node.addr, &req, timeout);
					
					for resp in rx {
						if tx.send((node.clone(), resp.clone())).is_err() {
							*(is_rx_dead.lock().unwrap()) = true;
						}

						if resp == Message::Timeout {
							break;
						}
					}
					sem.release();

					Ok(())
				});
			}

			Ok(())
		});
//		await!(Timeout::new(Duration::from_millis(timeout as u64), &self.handle).unwrap());

		rx
	}
}

impl Iterator for Server {
	type Item = (SocketAddr, Message);

	fn next(&mut self) -> Option<Self::Item> {
		let mut buf = [0; 64*1024];

		loop {
			let (len, src) = self.sock.recv_from(&mut buf).unwrap();
			let src = utils::ip4or6(src);
			let msg = &buf[..len];

			let msg:Result<Message,_> = deserialize(msg);

			debug!("got {:?}", msg);

			// dispatch responses
			match msg {
				Ok(Message::Ping(_))
				| Ok(Message::FindNode(_))
				| Ok(Message::FindValue(_))
				| Ok(Message::Store(_))
				| Ok(Message::Listen(_))
				| Ok(Message::Timeout)
				| Err(_) => (),

				Ok(ref resp @ Message::Pong(_))
				| Ok(ref resp @ Message::FoundNode(_))
				| Ok(ref resp @ Message::FoundValue(_)) => {
					let key = (src, *resp.cookie().unwrap());
					let pending = self.pending_requests.lock().unwrap();
					
					match (*pending).get(&key) {
						None => (),
						Some(tx) => ignore(tx.send(resp.clone())),
					}
				},
			}

			match msg {
				Err(_) | Ok(Message::Timeout) => (),
				Ok(r) => return Some((src, r)),
			}
		}
	}
}
