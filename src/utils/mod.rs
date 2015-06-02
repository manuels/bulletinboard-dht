mod take_until;
pub mod semaphore;

use std::net::{SocketAddr,SocketAddrV4};

pub fn ignore<R,E>(res: Result<R,E>) {
	match res {
		_ => ()
	}
}

pub fn ip4or6(addr: SocketAddr) -> SocketAddr {
	match addr {
		SocketAddr::V4(addr) => SocketAddr::V4(addr),
		SocketAddr::V6(addr) => {
				match addr.ip().to_ipv4() {
						None => SocketAddr::V6(addr),
						Some(ip) => SocketAddr::V4(SocketAddrV4::new(ip, addr.port()))
				}
		}
	}
}
