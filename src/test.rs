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

	let mut kad1 = Kademlia::bootstrap("0.0.0.0:10001", vec![super_addr], Some(ones.clone()));
	let mut kad2 = Kademlia::bootstrap("0.0.0.0:10002", vec![super_addr], Some(ones.clone()));

	kad1.put(zeros.clone(), vec![1,2,3]).unwrap();
	kad2.put(zeros.clone(), vec![4,5,6]).unwrap();
	kad1.put(zeros.clone(), vec![7,8,9]).unwrap();

	let mut result = kad1.get(zeros.clone());
	let mut result = kad1.get(zeros);
	result.sort_by(|a,b| a.cmp(b));
	result.dedup();
	assert_eq!(result, vec![vec![4,5,6], vec![7,8,9]]);
}
