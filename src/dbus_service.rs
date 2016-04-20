use std::borrow::Cow;

use dbus::{Connection,BusType,NameFlag,ConnectionItem,MessageItem};
use dbus::obj::{ObjectPath,Method,Argument,Interface};

use crypto::digest::Digest;
use crypto::sha1::Sha1;

use kademlia::Kademlia;

fn message_item_to_string(item: MessageItem) -> Result<String, (&'static str, String)> {
	match item {
		MessageItem::Str(string) => Ok(string),
		_ => {
			let err = format!("Cannot convert argument to string");
			Err(("org.manuel.Intercom.Invalid", err))
		}
	}
}

fn message_item_to_byte_vec(item: MessageItem)
	-> Result<Vec<u8>, (&'static str, String)>
{
	match item {
		MessageItem::Array(ref data, ref t) if t == "y" => {
			let res = data.iter().map(|i|
				match i {
					&MessageItem::Byte(b) => b,
					_  => unreachable!(),
				}
			);
			Ok(res.collect())
		},
		_ => {
			let err = format!("Cannot convert argument (type'{}') to bytearray.", item.type_sig());
			Err(("org.manuel.Intercom.Invalid", err))
		}
	}
}

fn byte_vec_to_message_item(vec: Vec<u8>) -> MessageItem {
	let items = vec.iter().map(|b| {
		MessageItem::Byte(*b)
	}).collect();

	MessageItem::Array(items, Cow::Borrowed("y"))
}

fn hash(app_id: String, data: &[u8]) -> Vec<u8> {
	let mut hasher = Sha1::new();

	let mut output = vec![0x0; hasher.output_bytes()];
	hasher.input(app_id.as_bytes());
	hasher.input(data);
	hasher.result(&mut output[..]);

	output
}

fn dht_get(kad: Kademlia, app_id: MessageItem, key: MessageItem)
	-> Result<Vec<MessageItem>, (&'static str, String)> 
{
	let app_id = try!(message_item_to_string(app_id));
	let key = try!(message_item_to_byte_vec(key));
	let hash_key = hash(app_id, &key);

	let items:Vec<MessageItem> = kad.get(hash_key).into_iter()
		.map(byte_vec_to_message_item)
		.collect();

	debug!("values len={}", items.len());
	let values = MessageItem::Array(items, Cow::Borrowed("ay"));
	Ok(vec![values])
}

fn dht_put(mut kad: Kademlia, app_id: MessageItem, key: MessageItem, value: MessageItem)
	-> Result<Vec<MessageItem>, (&'static str, String)>
{
	let app_id = try!(message_item_to_string(app_id));
	let key   = try!(message_item_to_byte_vec(key));
	let value = try!(message_item_to_byte_vec(value));
	let hash_key = hash(app_id, &key);

	kad.put(hash_key, value)
		.map(|_| vec![])
		.map_err(|_| ("org.manuel.Intercom.PutFailed", "Put failed".to_string()))
}

pub fn dbus(kad: Kademlia, dbus_name: &'static str) {
	let c = Connection::get_private(BusType::Session).unwrap();
	c.register_name(dbus_name, NameFlag::ReplaceExisting as u32).unwrap();

	let mut o = ObjectPath::new(&c, "/", true);
	o.insert_interface("org.manuel.BulletinBoard", Interface::new(
		vec![
			Method::new("Get",
				vec![Argument::new("app_id", "s"), Argument::new("key", "ay")],
				vec![Argument::new("value", "aay")],
				Box::new(|msg| {
					let app_id = try!(msg.get_items().get(0).ok_or(("org.manuel.BulletinBoard.Invalid", "Invaild app_id".to_string()))).clone();
					let key = try!(msg.get_items().get(1).ok_or(("org.manuel.BulletinBoard.Invaild", "Invalid key".to_string()))).clone();
					dht_get(kad.clone(), app_id, key)
				})
			),
			Method::new("Put",
				vec![Argument::new("app_id", "s"), Argument::new("key", "ay"), Argument::new("value", "ay")],
				vec![],
				Box::new(|msg| {
					let app_id = try!(msg.get_items().get(0).ok_or(("org.manuel.BulletinBoard.Invaild", "Invaild app_id".to_string()))).clone();
					let key = try!(msg.get_items().get(1).ok_or(("org.manuel.BulletinBoard.Invaild", "Invalid key".to_string()))).clone();
					let value = try!(msg.get_items().get(2).ok_or(("org.manuel.BulletinBoard.Invaild", "Invalid value".to_string()))).clone();
					dht_put(kad.clone(), app_id, key, value)
				})
			),
		],
		vec![],
		vec![]
	));
	o.set_registered(true).unwrap();

	const TIMEOUT_MS:i32 = 60000;
	for n in c.iter(TIMEOUT_MS) {
		match n {
			ConnectionItem::MethodCall(mut m) => {
				o.handle_message(&mut m);
			},
			_ => {},
		}
	}
}

#[cfg(test)]
mod tests {
	use std::thread::{sleep,spawn};
	use std::time::Duration;

	use dbus::{Connection,BusType,Message,MessageItem};

	use node::NODEID_BYTELEN;
	use kademlia::Kademlia;

	use super::byte_vec_to_message_item;
	use super::message_item_to_byte_vec;

	#[test]
	fn test() {
		let app_id = "test".to_string();

		let zeros = vec![0x00; NODEID_BYTELEN];
		let ones = vec![0xFF; NODEID_BYTELEN];

		let super_addr = ("127.0.0.1", 20000);
		let _ = Kademlia::new_supernode(super_addr, Some(zeros.clone()));

		let kad = Kademlia::bootstrap("127.0.0.1:20001", vec![super_addr], Some(ones.clone()));

		let dbus_name = "org.manuel.BulletinBoardTest1";
		let name = dbus_name.clone();
		spawn(move || {
			super::dbus(kad, name);
		});

		sleep(Duration::from_millis(500));
		dbus_put(dbus_name.clone(), &app_id, "foo".bytes().collect(), "bar".bytes().collect());
		
		let actual = dbus_get(dbus_name.clone(), &app_id, "foo".bytes().collect());
		
		let expected:Vec<u8> = "bar".bytes().collect();
		assert_eq!(actual, vec![expected]);

		let actual = dbus_get(dbus_name.clone(), &app_id, "emtpy".bytes().collect());
		let expected:Vec<Vec<u8>> = vec![];
		assert_eq!(actual, expected);
		
	}

	fn dbus_put(dbus_name: &'static str, app_id: &String, key: Vec<u8>, value: Vec<u8>) {
		let c = Connection::get_private(BusType::Session).unwrap();
		let mut m = Message::new_method_call(dbus_name, "/", "org.manuel.BulletinBoard", "Put").unwrap();
		m.append_items(&[MessageItem::Str(app_id.clone()), byte_vec_to_message_item(key), byte_vec_to_message_item(value)]);

		c.send_with_reply_and_block(m, 30000).unwrap();
	}

	fn dbus_get(dbus_name: &'static str, app_id: &String, key: Vec<u8>) -> Vec<Vec<u8>> {
		let c = Connection::get_private(BusType::Session).unwrap();
		let mut m = Message::new_method_call(dbus_name, "/", "org.manuel.BulletinBoard", "Get").unwrap();
		m.append_items(&[MessageItem::Str(app_id.clone()), byte_vec_to_message_item(key)]);

		let r = c.send_with_reply_and_block(m, 30000).unwrap();
		let mut reply = r.get_items();
		let res = reply.pop().unwrap();

		if let MessageItem::Array(values, _) = res {
			values.into_iter().map(|v| message_item_to_byte_vec(v).unwrap()).collect()
		} else {
			unreachable!()
		}
	}
}
