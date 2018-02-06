use std::net::SocketAddr;

use kademlia::NodeId;
use kademlia::Key;
use kademlia::Cookie;

pub struct Node(pub NodeId, pub SocketAddr);

pub enum Message {
    Request(NodeId, Cookie, Request),
    Response(NodeId, Cookie, Response),
}

#[derive(PartialEq)]
pub enum Response {
    Pong,
    FoundNode(Node),
    FoundValue(Vec<u8>)
}

pub enum Request {
    Ping,
    FindNode(NodeId),
    FindValue(Key),
    Store(Key, Vec<u8>),
}
