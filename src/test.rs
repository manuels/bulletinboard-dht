#![allow(unused_variables)]

use env_logger;

use node::NODEID_BYTELEN;
use kademlia::Kademlia;

#[test]
fn test() {
	env_logger::init().unwrap();

	let zeros = vec![0x00; NODEID_BYTELEN];
	let ones = vec![0xFF; NODEID_BYTELEN];

	let super_addr = ("127.0.0.1", 10000);
	let kad_super = Kademlia::new_supernode(super_addr, Some(zeros.clone()));

	let mut kad = Kademlia::bootstrap("0.0.0.0:10001", vec![super_addr], Some(ones.clone()));


	kad.put(zeros.clone(), vec![1,2,3]).unwrap();
	//sleep_ms(5000);
	assert_eq!(kad.get(zeros), vec![vec![1,2,3]]);
}
