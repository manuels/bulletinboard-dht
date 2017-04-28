#![allow(unused_variables)]

use env_logger;

use node::NODEID_BYTELEN;
use kademlia::Kademlia;

use std::thread::{spawn,sleep};
use std::time::Duration;

#[test]
fn test() {
	let _ = env_logger::init();

	let zeros = [0x00; NODEID_BYTELEN];
	let ones = [0xFF; NODEID_BYTELEN];

	let super_addr = ("127.0.0.1", 30000);
	let kad_super = Kademlia::new_supernode(super_addr, Some(zeros.clone()));

	let mut kad1 = Kademlia::bootstrap("0.0.0.0:30001", vec![super_addr], Some(ones.clone()));
	let mut kad2 = Kademlia::bootstrap("0.0.0.0:30002", vec![super_addr], Some(ones.clone()));

	kad1.put(zeros.clone(), vec![1,2,3]).unwrap();
	kad2.put(zeros.clone(), vec![4,5,6]).unwrap();
	kad1.put(zeros.clone(), vec![7,8,9]).unwrap();

	let result = kad1.get(zeros.clone());
	let mut result = kad1.get(zeros);
	result.sort_by(|a,b| a.cmp(b));
	result.dedup();
	assert_eq!(result, vec![vec![4,5,6], vec![7,8,9]]);
}


#[test]
fn test_concurrent() {
	let _ = env_logger::init();

	let zeros = [0x00; NODEID_BYTELEN];
	let zeros1 = zeros.clone();
	let ones = [0xFF; NODEID_BYTELEN];

	let super_addr = ("127.0.0.1", 40000);
	let kad_super = Kademlia::new_supernode(super_addr, Some(zeros.clone()));

	let mut kad1 = Kademlia::bootstrap("0.0.0.0:40001", vec![super_addr], Some(ones.clone()));
	let kad2 = Kademlia::bootstrap("0.0.0.0:40002", vec![super_addr], Some(ones.clone()));

	let mut kad11 = kad1.clone();
	spawn(move || {
		kad11.put(zeros1.clone(), vec![1,2,3]).unwrap();
	});
	kad1.put(ones.clone(), vec![4,5,6]).unwrap();

	sleep(Duration::from_millis(500));
	let result = kad1.get(zeros.clone());
	assert_eq!(result, vec![vec![1,2,3]]);
	
	let result = kad1.get(ones.clone());
	assert_eq!(result, vec![vec![4,5,6]]);
}
