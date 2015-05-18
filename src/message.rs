use node::{Node, NodeId};

pub const COOKIE_BYTELEN:usize = 160/8;

//pub type Cookie = [u8; COOKIE_LEN/8];
pub type Cookie = Vec<u8>;

#[derive(RustcDecodable, RustcEncodable, PartialEq, Clone, Debug)]
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

#[derive(RustcDecodable, RustcEncodable, PartialEq, Clone, Debug)]
pub struct Ping {
	pub sender_id: NodeId,
	pub cookie: Cookie,
}

#[derive(RustcDecodable, RustcEncodable, PartialEq, Clone, Debug)]
pub struct Pong {
	pub sender_id: NodeId,
	pub cookie: Cookie,
}

#[derive(RustcDecodable, RustcEncodable, PartialEq, Clone, Debug)]
pub struct FindNode {
	pub sender_id: NodeId,
	pub cookie:    Cookie,
	pub key:       NodeId,
}

#[derive(RustcDecodable, RustcEncodable, PartialEq, Clone, Debug)]
pub struct FindValue {
	pub sender_id: NodeId,
	pub cookie:    Cookie,
	pub key:       NodeId,
}

#[derive(RustcDecodable, RustcEncodable, PartialEq, Clone, Debug)]
pub struct FoundNode {
	pub sender_id: NodeId,
	pub cookie:    Cookie,
	pub nodes:     Vec<Node>,
}

#[derive(RustcDecodable, RustcEncodable, PartialEq, Clone, Debug)]
pub struct FoundValue {
	pub sender_id: NodeId,
	pub cookie:    Cookie,
	pub values:    Vec<Vec<u8>>,
}

#[derive(RustcDecodable, RustcEncodable, PartialEq, Clone, Debug)]
pub struct Store {
	pub sender_id: NodeId,
	pub cookie:    Cookie,
	pub key:       NodeId,
	pub value:     Vec<u8>,
}
