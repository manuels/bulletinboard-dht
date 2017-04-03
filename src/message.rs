use std::ops::Deref;

use rustc_serialize::base64;
use rustc_serialize::base64::{ToBase64,FromBase64};
use rustc_serialize::{Encodable,Decodable,Encoder,Decoder};

use node::{Node, NodeId};

use json;

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
	pub sender_id:  NodeId,
	pub cookie:     Cookie,
	pub node_count: usize,
	pub node:       Node,
}

#[derive(RustcDecodable, RustcEncodable, PartialEq, Clone, Debug)]
pub struct FoundValue {
	pub sender_id:   NodeId,
	pub cookie:      Cookie,
	pub value_count: usize,
	pub value:       Value,
}

#[derive(RustcDecodable, RustcEncodable, PartialEq, Clone, Debug)]
pub struct Store {
	pub sender_id: NodeId,
	pub cookie:    Cookie,
	pub key:       NodeId,
	pub value:     Value,
}

#[derive(PartialEq, Clone, Debug)]
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

impl Encodable for Value {
	fn encode<E: Encoder>(&self, enc: &mut E) -> Result<(), E::Error> {
		let base64 = self.data.to_base64(base64::STANDARD);

		enc.emit_str(&base64[..])
	}
}

impl Decodable for Value {
	fn decode<D: Decoder>(dec: &mut D) -> Result<Self, D::Error> {
		let base64 = try!(dec.read_str());
		let data = try!(base64.from_base64()
			.map_err(|_| dec.error("error decoding base64 Value")));

		Ok(Value::new(data))
	}
}

#[test]
fn test_value_coding() {
	let actual = Value::new(vec![1,2,3]);

	let encoded = json::encode(&actual).unwrap();
	warn!("{:?}", encoded);
	let expected: Value = json::decode(&encoded).unwrap();

	assert_eq!(actual, expected);
}

