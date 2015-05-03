use std::net::UdpSocket;
use std::io;
use std::io::{Cursor};
use std::io::{Read,Write};
use std::net::{SocketAddr,SocketAddrV4,SocketAddrV6};
use std::net::{Ipv4Addr,Ipv6Addr};

use rand;
use byteorder::{BigEndian, WriteBytesExt, ReadBytesExt};

use server::{ProtocolTrait, Cookie, Message, Protocol};
use peer::{Id, ID_LEN, Peer, WriteIdExt, ReadIdExt};

pub struct Pinboard;

const MAGIC: [u8;2] = [0x23,0x45];
const IPV4: u8 = 0x4;
const IPV6: u8 = 0x6;

const PING: u8 = 0x01;
const PONG: u8 = 0x02;
const FIND_NODE:   u8 = 0x03;
const FOUND_NODE:  u8 = 0x04;
const FIND_VALUE:  u8 = 0x05;
const FOUND_VALUE: u8 = 0x06;
const STORE:       u8 = 0x07;

const COOKIE_LEN: usize = 4;

impl Pinboard {
	fn generate_cookie(len: usize) -> Cookie {
		(0..len).map(|_| rand::random::<u8>()).collect()
	}
}

impl ProtocolTrait for Pinboard {
	fn request_ping(sock: &UdpSocket, own_id: &Id, peer: &Peer)
		-> io::Result<(Cookie, usize)>
	{
		let cookie = Self::generate_cookie(COOKIE_LEN);
		let mut msg = vec![];
		{
			let mut cur = Cursor::new(&mut msg[..]);

			try!(cur.write(&MAGIC[..]));
			try!(cur.write_u8(PING));
			try!(cur.write(&cookie[..]));
			try!(cur.write_id(own_id));
			try!(cur.flush());
		}

		let len = try!(sock.send_to(&msg[..], peer.addr()));
		Ok((cookie, len))
	}

	fn respond_pong(sock: &UdpSocket, own_id: &Id, peer: &Peer, cookie: &Cookie)
		-> io::Result<usize>
	{
		let mut msg = vec![];
		{
			let mut cur = Cursor::new(&mut msg[..]);

			try!(cur.write(&MAGIC[..]));
			try!(cur.write_u8(PONG));
			try!(cur.write(cookie));
			try!(cur.write_id(own_id));
			try!(cur.flush());
		}

		sock.send_to(&msg[..], peer.addr())
	}

	fn request_find_node(sock: &UdpSocket, own_id: &Id, peer: &Peer, find_id: &Id)
		-> io::Result<(Cookie, usize)>
	{
		let cookie = Self::generate_cookie(COOKIE_LEN);
		let mut msg = vec![];
		{
			let mut cur = Cursor::new(&mut msg[..]);

			try!(cur.write(&MAGIC[..]));
			try!(cur.write_u8(FIND_NODE));
			try!(cur.write(&cookie[..]));
			try!(cur.write_id(own_id));
			try!(cur.write_id(find_id));
			try!(cur.flush());
		}

		let len = try!(sock.send_to(&msg[..], peer.addr()));
		Ok((cookie, len))
	}

	fn respond_find_node(sock: &UdpSocket, peer: &Peer,
	                     cookie: &Cookie, peer_list: &Vec<Peer>)
		-> io::Result<usize>
	{
		let mut msg = vec![];
		{
			let mut cur = Cursor::new(&mut msg[..]);

			try!(cur.write(&MAGIC[..]));
			try!(cur.write_u8(FOUND_NODE));
			try!(cur.write(&cookie[..]));
			for p in peer_list {
				try!(cur.write_id(p.id()));
				match p.addr() {
					&SocketAddr::V4(addr) => {
						try!(cur.write_u8(IPV4));
						try!(cur.write(&addr.ip().octets()[..]));
						try!(cur.write_u16::<BigEndian>(addr.port()));
					},
					&SocketAddr::V6(addr) => {
						try!(cur.write_u8(IPV6));
						for s in addr.ip().segments().iter() {
							try!(cur.write_u16::<BigEndian>(*s));
						}
						try!(cur.write_u16::<BigEndian>(addr.port()));
					},
				}
				/*
				match p.addr().ip() {
					IpAddr::V4(addr) => {
						try!(cur.write_u8(IPV4));
						try!(cur.write(&addr.octets()[..]));
					},
					IpAddr::V6(addr) => {
						try!(cur.write_u8(IPV6));
						for s in addr.segments().iter() {
							try!(cur.write_u16::<BigEndian>(*s));
						}
					},
				}
				try!(cur.write_u16::<BigEndian>(p.addr().port()));
				*/
			}
			try!(cur.flush());
		}

		let len = try!(sock.send_to(&msg[..], peer.addr()));
		Ok(len)
	}

	fn request_find_value(sock: &UdpSocket, own_id: &Id, peer: &Peer, find_id: &Id)
		-> io::Result<(Cookie, usize)>
	{
		let cookie = Self::generate_cookie(COOKIE_LEN);
		let mut msg = vec![];
		{
			let mut cur = Cursor::new(&mut msg[..]);

			try!(cur.write(&MAGIC[..]));
			try!(cur.write_u8(FIND_VALUE));
			try!(cur.write(&cookie[..]));
			try!(cur.write_id(own_id));
			try!(cur.write_id(find_id));
			try!(cur.flush());
		}

		let len = try!(sock.send_to(&msg[..], peer.addr()));
		Ok((cookie, len))
	}

	fn respond_find_value(sock: &UdpSocket, peer: &Peer, own_id: &Id,
	                      cookie: &Cookie, result: Result<Vec<&Vec<u8>>,Vec<Peer>>)
		-> io::Result<usize>
	{
		match result {
			Err(peer_list) => Self::respond_find_node(sock, peer, cookie, &peer_list),
			Ok(value_list) => {
				let mut msg = vec![];
				{
					let mut cur = Cursor::new(&mut msg[..]);

					try!(cur.write(&MAGIC[..]));
					try!(cur.write_u8(FOUND_VALUE));
					try!(cur.write_id(own_id));
					try!(cur.write(&cookie[..]));
					for v in value_list.iter() {
						try!(cur.write_u8(v.len() as u8));
						try!(cur.write(v));
					}
					try!(cur.flush());
				}

				sock.send_to(&msg[..], peer.addr())
			}
		}
	}

	fn request_store(sock: &UdpSocket, own_id: &Id, peer: &Peer, key: &Id, value: &Vec<u8>)
		-> io::Result<usize>
	{
		let mut msg = vec![];
		{
			let mut cur = Cursor::new(&mut msg[..]);

			try!(cur.write(&MAGIC[..]));
			try!(cur.write_u8(STORE));
			try!(cur.write_id(own_id));
			try!(cur.write_id(key));
			try!(cur.write(value));
			try!(cur.flush());
		}

		sock.send_to(&msg[..], peer.addr())
	}

	fn parse(buf: &[u8]) -> io::Result<Message> {
		let mut cur = Cursor::new(&buf[..]);

		if try!(cur.read_u8()) != MAGIC[0] ||
		   try!(cur.read_u8()) != MAGIC[1] {
			return Ok(Message::Undefined);
		}

		match try!(cur.read_u8()) {
			PING => Self::parse_ping(&mut cur),
			PONG => Self::parse_pong(&mut cur),
			STORE  => Self::parse_store(&mut cur),
			FIND_NODE  => Self::parse_find_node(&mut cur),
			FIND_VALUE => Self::parse_find_value(&mut cur),
			FOUND_NODE  => Self::parse_found_node(&mut cur),
			FOUND_VALUE => Self::parse_found_value(&mut cur),
			_ => Ok(Message::Undefined),
		}
	}
}

impl Pinboard {
	fn parse_ping(cur: &mut Cursor<&[u8]>) -> io::Result<Message> {
		let mut cookie = [0; COOKIE_LEN];
		let mut id = Id::new([0; ID_LEN/8]);

		let len = try!(cur.read(&mut cookie[..]));
		if len != cookie.len() {
			return Ok(Message::Undefined);
		}
		try!(cur.read_id(&mut id));

		Ok(Message::Ping(cookie.to_vec(), id))
	}

	fn parse_pong(cur: &mut Cursor<&[u8]>) -> io::Result<Message> {
		let mut cookie = [0; COOKIE_LEN];
		let mut id = Id::new([0; ID_LEN/8]);

		let len = try!(cur.read(&mut cookie[..]));
		if len != cookie.len() {
			return Ok(Message::Undefined);
		}
		try!(cur.read_id(&mut id));

		Ok(Message::Pong(cookie.to_vec(), id))
	}

	fn parse_find_node(cur: &mut Cursor<&[u8]>) -> io::Result<Message> {
		let mut cookie = [0; COOKIE_LEN];
		let mut id = Id::new([0; ID_LEN/8]);
		let mut find_id = Id::new([0; ID_LEN/8]);

		let len = try!(cur.read(&mut cookie[..]));
		if len != cookie.len() {
			return Ok(Message::Undefined);
		}
		try!(cur.read_id(&mut id));
		try!(cur.read_id(&mut find_id));

		Ok(Message::FindNode(cookie.to_vec(), id, find_id))
	}

	fn parse_found_value(cur: &mut Cursor<&[u8]>) -> io::Result<Message> {
		let mut cookie = [0; COOKIE_LEN];
		let mut id = Id::new([0; ID_LEN/8]);

		let len = try!(cur.read(&mut cookie[..]));
		if len != cookie.len() {
			return Ok(Message::Undefined);
		}
		try!(cur.read_id(&mut id));

		let mut values = vec![];
		let mut len = cur.read_u8();
		while len.is_ok() {
			let expected_len = len.unwrap() as usize;
			let mut v = vec![0; expected_len];

			let actual_len = try!(cur.read(&mut v));
			if actual_len != expected_len {
				return Ok(Message::Undefined);
			}

			values.push(v);
			len = cur.read_u8();
		}

		Ok(Message::FoundValue(cookie.to_vec(), id, values))
	}

	fn parse_found_node(cur: &mut Cursor<&[u8]>) -> io::Result<Message> {
		let mut cookie = [0; COOKIE_LEN];
		let mut id = Id::new([0; ID_LEN/8]);

		let len = try!(cur.read(&mut cookie[..]));
		if len != cookie.len() {
			return Ok(Message::Undefined);
		}

		let mut peer_list = vec![];
		while cur.read_id(&mut id).is_ok() {
			match try!(cur.read_u8()) {
				IPV4 => {
					let mut octets = [0u8; 4];
					let len = try!(cur.read(&mut octets[..]));
					if len != octets.len() {
						return Ok(Message::Undefined);
					}

					let ip = Ipv4Addr::new(octets[0], octets[1], octets[2], octets[3]);
					let port = try!(cur.read_u16::<BigEndian>());

					let addr = SocketAddrV4::new(ip, port);
					let peer = Peer::new(id.clone(), SocketAddr::V4(addr), Protocol::Pinboard);
					peer_list.push(peer);
				}
				IPV6 => {
					let mut segments = [0u16; 8];
					for s in segments.iter_mut() {
						*s = try!(cur.read_u16::<BigEndian>());
					}
					let a = segments[0];
					let b = segments[1];
					let c = segments[2];
					let d = segments[3];
					let e = segments[4];
					let f = segments[5];
					let g = segments[6];
					let h = segments[7];

					let ip = Ipv6Addr::new(a,b,c,d,e,f,g,h);
					let port = try!(cur.read_u16::<BigEndian>());

					let addr = SocketAddrV6::new(ip, port, 0, 0);
					let peer = Peer::new(id.clone(), SocketAddr::V6(addr), Protocol::Pinboard);
					peer_list.push(peer);
				}
				_ => return Ok(Message::Undefined)
			}
		}

		Ok(Message::FoundNode(cookie.to_vec(), id, peer_list))
	}

	fn parse_find_value(cur: &mut Cursor<&[u8]>) -> io::Result<Message> {
		let mut cookie = [0; COOKIE_LEN];
		let mut id = Id::new([0; ID_LEN/8]);
		let mut find_id = Id::new([0; ID_LEN/8]);

		try!(cur.read(&mut cookie[..]));
		try!(cur.read_id(&mut id));
		try!(cur.read_id(&mut find_id));

		Ok(Message::FindValue(cookie.to_vec(), id, find_id))
	}

	fn parse_store(cur: &mut Cursor<&[u8]>) -> io::Result<Message> {
		let mut id = Id::new([0; ID_LEN/8]);
		let mut key = Id::new([0; ID_LEN/8]);
		let mut value = vec![];

		try!(cur.read_id(&mut id));
		try!(cur.read_id(&mut key));
		try!(cur.read_to_end(&mut value));

		Ok(Message::Store(id, key, value))
	}
}
