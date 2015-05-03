#[cfg(test)]
use std::thread::{spawn,sleep_ms};

use std::sync::{Arc,Mutex,Condvar};

#[derive(Clone)]
pub struct Semaphore {
	count: Arc<(Mutex<isize>, Condvar)>,
	borrowed: isize,
}

impl Semaphore {
	pub fn new(count: isize) -> Semaphore {
		Semaphore {
			count:    Arc::new((Mutex::new(count), Condvar::new())),
			borrowed: 0,
		}
	}

	pub fn acquire(&self) -> Semaphore {
		let &(ref lock, ref cvar) = &*self.count;

		let mut count = lock.lock().unwrap();
		while *count == 0 {
			count = cvar.wait(count).unwrap();
		}
		*count -= 1;

		if *count < 0 {
			unreachable!();
		}

		let mut res = self.clone();
		res.borrowed = 1;
		res
	}
}

impl Drop for Semaphore {
	fn drop(&mut self) {
		let &(ref lock, ref cvar) = &*self.count;

		let mut count = lock.lock().unwrap();
		*count += self.borrowed;

		cvar.notify_one();
	}
}

#[test]
fn it_works() {
	let sem = Semaphore::new(5);

	for _ in 0..10 {
		let s = sem.acquire();
		spawn(move || {
			let s = s;
			sleep_ms(250);
			drop(s);
		});
		sleep_ms(125);
	}
	sem.acquire();
}
