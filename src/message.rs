use std::ops::Deref;

use node::{Node, NodeId};

pub const COOKIE_BYTELEN:usize = 160/8;

//pub type Cookie = [u8; COOKIE_LEN/8];
pub type Cookie = Vec<u8>;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum Message {
		Ping(Ping),
		Pong(Pong),
		FindNode(FindNode),
		FoundNode(FoundNode),
		FindValue(FindValue),
		FoundValue(FoundValue),
		Store(Store),
		Timeout,
}

impl Message {
	pub fn cookie(&self) -> Option<&Cookie> {
		match self {
			&Message::Ping(ref r) => Some(&r.cookie),
			&Message::Pong(ref r) => Some(&r.cookie),
			&Message::FindNode(ref r) => Some(&r.cookie),
			&Message::FoundNode(ref r) => Some(&r.cookie),
			&Message::FindValue(ref r) => Some(&r.cookie),
			&Message::FoundValue(ref r) => Some(&r.cookie),
			&Message::Store(ref r) => Some(&r.cookie),
			&Message::Timeout => None,
		}
	}

	pub fn sender_id(&self) -> Option<NodeId> {
		match self {
			&Message::Ping(ref r) => Some(r.sender_id.clone()),
			&Message::Pong(ref r) => Some(r.sender_id.clone()),
			&Message::FindNode(ref r) => Some(r.sender_id.clone()),
			&Message::FoundNode(ref r) => Some(r.sender_id.clone()),
			&Message::FindValue(ref r) => Some(r.sender_id.clone()),
			&Message::FoundValue(ref r) => Some(r.sender_id.clone()),
			&Message::Store(ref r) => Some(r.sender_id.clone()),
			&Message::Timeout => None,
		}
	}
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Ping {
	pub sender_id: NodeId,
	pub cookie: Cookie,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Pong {
	pub sender_id: NodeId,
	pub cookie: Cookie,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct FindNode {
	pub sender_id: NodeId,
	pub cookie:    Cookie,
	pub key:       NodeId,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct FindValue {
	pub sender_id: NodeId,
	pub cookie:    Cookie,
	pub key:       NodeId,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct FoundNode {
	pub sender_id:  NodeId,
	pub cookie:     Cookie,
	pub node_count: usize,
	pub node:       Node,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct FoundValue {
	pub sender_id:   NodeId,
	pub cookie:      Cookie,
	pub value_count: usize,
	pub value:       Value,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Store {
	pub sender_id: NodeId,
	pub cookie:    Cookie,
	pub key:       NodeId,
	pub value:     Value,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Value {
	data: Vec<u8>
}

impl Value {
	pub fn new(data: Vec<u8>) -> Value {
		Value { data: data }
	}
}

impl Deref for Value {
	type Target = Vec<u8>;

	fn deref<'a>(&'a self) -> &'a Vec<u8> {
		&self.data
	}
}
