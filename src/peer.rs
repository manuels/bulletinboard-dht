use std::ops::BitAnd;
use std::ops::BitXor;
use std::io;
use std::io::{Result,Error};
use std::fmt;
use std::fmt::{Debug, Formatter};

use rand;

use server::Protocol;
use std::net::SocketAddr;

pub const ID_LEN:usize = 160;

#[derive(Eq,Hash)]
pub struct Id {
	val: [u8; ID_LEN/8]
}

impl Id {
	pub fn new(val: [u8; ID_LEN/8]) -> Id {
		Id {val: val}
	}

	pub fn distance_to(&self, id: &Id) -> usize {
		let dist = self.clone() ^ id.clone();

		for (i, x) in dist.val.iter().enumerate() {
			for j in 0..8 {
				let k = 8-j;

				if x & (2 << k) > 0 {
					return i*8 + k;
				}
			}
		}

		ID_LEN
	}

	pub fn generate() -> Id {
		let mut id = [0u8; ID_LEN/8];

		for i in 0..id.len() {
			id[i] = rand::random::<u8>();
		}

		Id::new(id)
	}
}

impl Debug for Id {
	fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
		let id:Vec<String> = self.val.iter()
			.map(|x| format!("{:x}", x))
			.collect();

		fmt.write_str(&id.concat()[..])
	}
}

impl BitAnd<Id> for Id {
	type Output = Id;

	fn bitand(self, rhs: Id) -> Self::Output {
		let mut res = Id::new([0u8; ID_LEN/8]);
		
		for (i,x) in self.val.iter().enumerate() {
			res.val[i] = x & rhs.val[i];
		}

		res
	}
}

impl BitXor<Id> for Id {
	type Output = Id;

	fn bitxor(self, rhs: Id) -> Self::Output {
		let mut res = [0u8; ID_LEN/8];

		for (i,(x,y)) in self.val.iter().zip(rhs.val.iter()).enumerate() {
			res[i] = x ^ y;
		}
		Id::new(res)
	}
}

impl PartialEq for Id {
	fn eq(&self, other: &Id) -> bool {
		for (i, x) in self.val.iter().enumerate() {
			if *x != other.val[i] {
				return false;
			}
		}
		true
	}
}

impl Clone for Id {
	fn clone(&self) -> Self {
		Id {
			val: self.val,
		}
	}
}

pub trait WriteIdExt: io::Write {
	fn write_id(&mut self, id: &Id) -> io::Result<()> {
		self.write(&id.val[..]).map(|_| ())
	}
}

pub trait ReadIdExt: io::Read {
	fn read_id(&mut self, id: &mut Id) -> io::Result<()> {
		let len = try!(self.read(&mut id.val[..]));
		if len != id.val.len() {
			return Err(io::Error::new(io::ErrorKind::Other, "EOF"));
		}
		Ok(())
	}
}

impl<'a> WriteIdExt for io::Cursor<&'a mut [u8]> {}
impl<'a> ReadIdExt for io::Cursor<&'a [u8]> {}

#[derive(Clone)]
pub struct Peer {
	id:       Id,
	addr:     SocketAddr,
	protocol: Protocol,
}

impl Debug for Peer {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
    	fmt.write_fmt(format_args!("Peer id={:?}", &self.id()))
    }
}

impl Peer {
	pub fn new(id: Id, addr: SocketAddr, protocol: Protocol) -> Peer {
		Peer {
			id:       id,
			addr:     addr,
			protocol: protocol,
		}
	}

	pub fn strictly_same_as(&self, other: &Peer) -> bool {
		self.id() == other.id() &&
			self.addr() == other.addr() &&
			self.protocol() == other.protocol()
	}

	pub fn id(&self) -> &Id {
		&self.id
	}

	pub fn protocol(&self) -> &Protocol {
		&self.protocol
	}

	pub fn addr(&self) -> &SocketAddr {
		&self.addr
	}

	pub fn store(&self, id: &Id, value: &Vec<u8>) -> Result<()> {
		unimplemented!()
	}

	pub fn find_node(&self, id: &Id) -> Result<Peer> {
		unimplemented!()
	}

	pub fn find_value(&self, id: &Id) -> Result<Vec<u8>> {
		unimplemented!()
	}
}
