#![macro_use]

use std::rc::Rc;
use std::cell::RefCell;
use std::io;
use std::net::SocketAddr;
use std::time::Duration;
use std::collections::HashMap;
use rand;
use futures::unsync::mpsc;
use futures::unsync::mpsc::Sender;
use futures::unsync::mpsc::Receiver;
use futures::unsync::oneshot;
use futures::prelude::*;
use tokio_core::reactor::Handle;
use tokio_core::net::UdpSocket;
use tokio_timer::Timer;

use Result;
use Error;
use ErrorKind;
use server;
use messages::Node;
use messages::Message;
use messages::Request;
use messages::Response;

/*
#[macro_export]
macro_rules! report {
    ( $x:expr ) => {
        ($x).map_err(|e| {
            error!("{:?}", e);
            ()
        })
    };
}
*/

pub fn report<F:Future<Item=(),Error=Error>>(future: F)
    -> impl Future<Item=(), Error=()>
{
    future.map_err(|e| {
        error!("{:?}", e);
    })
}


pub type Key = [u8; 20];
pub type NodeId = [u8; 20];
pub type Cookie = [u8; 20];

pub type ConnectionMap = HashMap<(SocketAddr, Cookie), Sender<Response>>;

const MAX_VALUE_LEN: usize = 1500;

struct Kademlia {
    handle: Handle,
    our_node_id: Rc<RefCell<NodeId>>,
    kbuckets: u8,
    connections: Rc<RefCell<ConnectionMap>>,
}

impl Kademlia {
    #[async]
    pub fn new(handle: Handle, bind_addr: SocketAddr) -> Result<Kademlia> {
        let supernodes = []; // TODO

        await!(Self::new_with_supernodes(handle, bind_addr, &supernodes))
    }

    #[async]
    pub fn new_with_supernodes(handle: Handle, bind_addr: SocketAddr, supernodes: &[SocketAddr]) -> Result<Kademlia> {
        let our_node_id = Rc::new(RefCell::new(rand::random()));

        let kbuckets = 0; // TODO KBuckets::new();
        for node in supernodes {
            // TODO
        }

        let kat = Kademlia {
            handle: handle.clone(),
            connections: Rc::new(RefCell::new(HashMap::new())),
            our_node_id,
            kbuckets
        };

        kat.spawn_udp_io();
        // TODO: kbuckets updating

        loop {
            let our_node_id = *kat.our_node_id.borrow();
            let nodes = await!(find_node(handle, kat.connections.clone(),
                outbound_tx, inbound_tx, inbound_rx,
                our_node_id, *kat.our_node_id.borrow()))?;

            if nodes.iter().any(|n| n.0 == our_node_id) {
                return Ok(kat);
            }
            *kat.our_node_id.borrow_mut() = rand::random();
        }
    }

    fn spawn_udp_io(&self, bind_addr: SocketAddr) -> Result<()> {
        let sock = UdpSocket::bind(bind_addr)?;
        let framed = sock.framed::<server::Codec>();
        let (udp_sink, udp_stream) = framed.split();

        let (outbound_tx, outbound_rx) = mpsc::channel();
        self.handle.spawn(outbound_rx.send_all(udp_sink));

        self.handle.spawn(server::receive_loop(self.handle.clone(), outbound_tx, udp_stream, self.connections.clone()));
    }

    pub fn put(&self, key: Key, value: Vec<u8>) -> impl Future<Item=(), Error=Error> {
        let handle = self.handle.clone();
        let our_node_id = self.our_node_id.borrow().clone();
        let (tx, rx, cookie) = self.register_connection();

        put(handle.clone(), tx, rx, cookie, our_node_id, key, value)
    }

    pub fn get(&self, key: Key) -> Result<Receiver<Vec<u8>>> {
        let handle = self.handle.clone();
        let (tx, rx) = self.new_connection();

        let request = Request::FindValue(key);

        let (_node_rx, value_rx) = find(handle, self.connections.clone(), tx, rx, self.our_node_id.clone(), request)?;
        Ok(value_rx)
    }

    fn new_connection(&self) -> (Sender<(SocketAddr, Request)>, Rc<RefCell<Sender<Response>>>, Receiver<Response>) {
        let (incoming_tx, incoming_rx) = mpsc::channel();
        let (outgoing_tx, outgoing_rx) = mpsc::channel();

        self.handle.spawn(outgoing_rx.forward(self.outgoing.clone()));

        let incoming_tx = Rc::new(RefCell::new(incoming_tx));
        (outgoing_tx, incoming_tx, incoming_rx)
    }
}

#[async]
fn ping(outbound_tx: Sender<(SocketAddr, Message)>, inbound_tx: Sender<Response>, inbound_rx: Receiver<Response>, our_node_id: NodeId, dst: SocketAddr) -> Result<bool> {
    let cookie = rand::random();
    // TODO register

    let req = Message::Request(our_node_id, cookie, Request::Ping);
    await!(outbound_tx.send((dst, req)))?;

    let duration = Duration::from_millis(1000);
    let inbound_rx = Timer::default().timeout_stream(inbound_rx, duration);

    let res: Result<bool> = do catch {
        #[async]
        for response in inbound_rx {
            if response == Response::Pong {
                return Ok(true)
            }
        }

        Ok(false)
    };

    match res {
        Err(e) if e.into().kind() == io::ErrorKind::TimedOut => Ok(false),
        res => res.into(),
    }
}

fn find(handle: Handle,
        connections: Rc<RefCell<ConnectionMap>>,
        mut outbound_tx: Sender<(SocketAddr, Message)>,
        inbound_tx: Sender<Response>,
        inbound_rx: Receiver<Response>,
        our_node_id: NodeId,
        request: Request)
    -> Result<(oneshot::Receiver<Vec<Node>>, Sender<Vec<u8>>)>
{
    let (result_nodes_tx, result_nodes_rx) = oneshot::channel();
    let (result_values_tx, result_values_rx) = mpsc::channel(10);
    let (request_nodes_tx, request_nodes_rx): (_, Receiver<Node>) = mpsc::channel(10);

    let duration = Duration::from_millis(500);
    let inbound_rx = Timer::default().timeout_stream(inbound_rx, duration);

    let send_future = async_block! {
        #[async]
        for node in request_nodes_rx {
            let addr: SocketAddr = node.1;
            let cookie: Cookie = rand::random();
            let request = Message::Request(our_node_id.clone(), cookie.clone(), request);

            {
                let c = connections.borrow_mut();
                c.insert((addr, cookie), inbound_tx.clone());
            }

            let res = await!(outbound_tx.send((addr, request)));
            if let Ok(tx) = res {
                outbound_tx = tx;
            } else {
                break;
            }
        }

        Ok(())
    };

    let recv_future = async_block! {
        let mut result_values_tx = Some(result_values_tx);

        let res:io::Result<_> = do catch {
            #[async]
            for msg in inbound_rx {
                match msg {
                    Response::FoundNode(node) => {
                        request_nodes_tx.add(node)
                    },
                    Response::FoundValue(value) => {
                        if let Some(tx) = result_values_tx {
                            if let Ok(tx) = await!(tx.send(value)) {
                                result_values_tx = Some(tx)
                            } else {
                                result_values_tx = None;
                            }
                        }
                    },
                };
            }

            Ok(())
        };

        if let Err(e) = res {
            if e.kind() != io::ErrorKind::TimedOut {
                return Err(e.into());
            }
        }

        result_nodes_tx.send(request_nodes_tx.closest())
    };

    handle.spawn(report(recv_future));
    handle.spawn(report(send_future));

    Ok((result_nodes_rx, result_values_tx))
}

#[async]
fn find_node(handle: Handle,
        connections: Rc<RefCell<ConnectionMap>>,
        outbound_tx: Sender<(SocketAddr, Message)>,
        inbound_tx: Sender<Response>,
        inbound_rx: Receiver<Response>,
        our_node_id: NodeId,
        node_id: NodeId)
    -> Result<Vec<Node>>
{
    let request = Request::FindNode(node_id);

    let (node_rx, _value_rx) = find(handle, connections, outbound_tx, inbound_tx,
        inbound_rx, our_node_id, request)?;
    await!(node_rx).map_err(|e| e.into())
}


#[async]
fn put(handle: Handle,
        connections: Rc<RefCell<ConnectionMap>>,
        mut outbound_tx: Sender<(SocketAddr, Message)>,
        inbound_tx: Sender<Response>,
        inbound_rx: Receiver<Response>,
        our_node_id: NodeId,
        key: Key,
        value: Vec<u8>)
    -> Result<()>
{
    if value.len() > MAX_VALUE_LEN {
        return Err(ErrorKind::ValueTooLong.into());
    }

    let request = Request::FindNode(key as NodeId);
    let node_list = await!(find_node(handle, connections, outbound_tx.clone(), inbound_tx,
        inbound_rx, our_node_id, key as NodeId))?;


    #[async]
    for node in node_list {
        let request = Message::Request(our_node_id, rand::random(), Request::Store(key, value));
        outbound_tx = await!(outbound_tx.send((node.addr, request)))?;
    }

    Ok(())
}
