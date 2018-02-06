use std::rc::Rc;
use std::cell::RefCell;
use std::io;
use std::net::SocketAddr;

use futures::prelude::*;
use futures::unsync::mpsc::Sender;
use futures::unsync::mpsc::Receiver;
use tokio_core::reactor::Handle;
use tokio_core::net::UdpCodec;

use Result;
use kademlia::Key;
use kademlia::NodeId;
use kademlia::Cookie;
use kademlia::ConnectionMap;
use messages::Message;
use messages::Request;
use messages::Response;

#[macro_use]
use kademlia::report;

pub struct Codec;

impl UdpCodec for Codec {
    type In = (SocketAddr, Message);
    type Out = (SocketAddr, Message);

    fn decode(&mut self, src: &SocketAddr, buf: &[u8]) -> io::Result<Self::In> {
        unimplemented!()
    	//let msg = deserialize(buf);
//    	(src, msg)
    }

    fn encode(&mut self, msg: Self::Out, buf: &mut Vec<u8>) -> SocketAddr {
        let (dst, msg) = msg;
        unimplemented!()
//    	let buf = serialize(&msg, Bounded(2048)).unwrap();
  //  	dst
	}
}

#[async]
pub fn receive_loop(handle: Handle,
                sink: Sender<(SocketAddr, Message)>,
                stream: Receiver<(SocketAddr, Message)>,
                our_node_id: Rc<RefCell<NodeId>>,
                connections: Rc<RefCell<ConnectionMap>>)
    -> Result<()>
{
    #[async]
    for (src, msg) in stream {
        let our_node_id = our_node_id.borrow().clone();
        let future = match msg {
		    Message::Request(sender_id, cookie, Request::Ping) => process_ping(sink.clone(), our_node_id, src, cookie),
		    Message::Request(sender_id, cookie, Request::FindNode(node_id)) => process_find_node(sink.clone(), our_node_id, src, cookie, node_id),
		    Message::Request(sender_id, cookie, Request::FindValue(key)) => process_find_value(sink.clone(), our_node_id, src, cookie, key),
		    Message::Response(sender_id, cookie, response) => process_response(connections.clone(), src, cookie, response),
        };
        handle.spawn(report(future));
    }

    Ok(())
}

#[async(boxed)]
fn process_find_node(mut sink: Sender<(SocketAddr, Message)>, our_node_id: NodeId, src: SocketAddr, cookie: Cookie, node_id: NodeId) -> Result<()> {
    for node in kbuckets.closest(node_id) {
        let response = Message::Response(our_node_id, cookie, node.addr);
        sink = await!(sink.send((src, response)))?;
    }

    Ok(())
}

#[async(boxed)]
fn process_find_value(sink: Sender<(SocketAddr, Message)>, our_node_id: NodeId, src: SocketAddr, cookie: Cookie, key: Key) -> Result<()> {
    use std::collections::HashMap;
    let storage = HashMap::new();
    if let Some(value) = storage.get(&key) {
        unimplemented!()
    } else {
        await!(process_find_node(sink, our_node_id, src, cookie, key as NodeId))
    }
}


#[async(boxed)]
fn process_ping(sink: Sender<(SocketAddr, Message)>, our_node_id: NodeId, src: SocketAddr, cookie: Cookie) -> Result<()> {
    let response = Message::Response(our_node_id, cookie, Response::Pong);
    let _ = await!(sink.send((src, response)))?;
    Ok(())
}

#[async(boxed)]
fn process_response(connections: Rc<RefCell<ConnectionMap>>,
        src: SocketAddr,
        cookie: Cookie,
        response: Response)
    -> Result<()>
{
    let item = { connections.borrow().get(&(src, cookie)) };
    if let Some(tx) = item {
        tx.clone();
        let _ = await!(tx.send(response))?;
    } else {
        debug!("Unknown request: {}/{:?}", src, cookie);
    }

    Ok(())
}