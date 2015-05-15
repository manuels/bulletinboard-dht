use env_logger;
use std::thread::sleep_ms;

use kademlia::Kademlia;

#[test]
fn test() {
	env_logger::init().unwrap();

	let super_addr = ("localhost", 10000);
	let kad_super = Kademlia::new_supernode(super_addr, None);

	let mut kad = Kademlia::bootstrap("localhost:10001", vec![super_addr], None);

	let zeros = vec![0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
		             0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00];

	kad.put(zeros.clone(), vec![1,2,3]).unwrap();
	sleep_ms(5000);
	assert_eq!(kad.get(zeros), Some(vec![vec![1,2,3]]));
}
