mod take_until;
pub mod semaphore;

pub fn ignore<R,E>(res: Result<R,E>) {
	match res {
		_ => ()
	}
}
