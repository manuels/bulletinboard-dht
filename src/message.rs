use std::fmt;
use std::ops::Deref;

use node::{Node, NodeId};

pub const COOKIE_BYTELEN:usize = 160/8;

pub type Cookie = [u8; COOKIE_BYTELEN];

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

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct Ping {
	pub sender_id: NodeId,
	pub cookie: Cookie,
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct Pong {
	pub sender_id: NodeId,
	pub cookie: Cookie,
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct FindNode {
	pub sender_id: NodeId,
	pub cookie:    Cookie,
	pub key:       NodeId,
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct FindValue {
	pub sender_id: NodeId,
	pub cookie:    Cookie,
	pub key:       NodeId,
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct FoundNode {
	pub sender_id:  NodeId,
	pub cookie:     Cookie,
	pub node_count: usize,
	pub node:       Node,
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct FoundValue {
	pub sender_id:   NodeId,
	pub cookie:      Cookie,
	pub value_count: usize,
	pub value:       Value,
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
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

pub fn enc_id(id: &NodeId) -> String {
    let start:String = id[..3].iter().map(|x| format!("{:02x}", x)).collect();
    start + "..."
}

pub fn enc_vec(id: &Vec<u8>) -> String {
    let start:String = id[..3].iter().map(|x| format!("{:02x}", x)).collect();
    start + "..."
}

impl fmt::Debug for Store {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "sender={}, cookie={}, key: {}, value_len: {}",
			enc_id(&self.sender_id), enc_id(&self.cookie), enc_id(&self.key), &self.value.data.len())
	}
}

impl fmt::Debug for Ping {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "sender={}, cookie={}",
			enc_id(&self.sender_id), enc_id(&self.cookie))
	}
}

impl fmt::Debug for Pong {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "sender={}, cookie={}",
			enc_id(&self.sender_id), enc_id(&self.cookie))
	}
}

impl fmt::Debug for FindNode {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "sender={}, cookie={}, key={}",
			enc_id(&self.sender_id), enc_id(&self.cookie), enc_id(&self.key))
	}
}

impl fmt::Debug for FindValue {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "sender={}, cookie={}, key={}",
			enc_id(&self.sender_id), enc_id(&self.cookie), enc_id(&self.key))
	}
}

impl fmt::Debug for FoundNode {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "sender={}, cookie={}, count={} {:?}",
			enc_id(&self.sender_id), enc_id(&self.cookie), self.node_count, self.node)
	}
}

impl fmt::Debug for FoundValue {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "sender={}, cookie={}, count={} {}",
			enc_id(&self.sender_id), enc_id(&self.cookie), self.value_count, enc_vec(&self.value))
	}
}

