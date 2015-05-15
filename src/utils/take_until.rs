/*
#[derive(Clone)]
pub struct TakeUntil<I, P> {
    iter: I,
    flag: bool,
    predicate: P,
}

fn take_until<I: Iterator, P>(iter: Iterator, predicate: P)
    -> TakeUntil<Iterator, P>
        where Iterator: Sized, P: FnMut(&Iterator::Item) -> bool
{
    TakeUntil {
        iter: iter,
        flag: false,
        predicate: predicate,
    }
}

impl<I: Iterator, P> Iterator for TakeUntil<I, P>
    where P: FnMut(&I::Item) -> bool
{
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<I::Item> {
        if self.flag {
            None
        } else {
            self.iter.next().and_then(|x| {
                if !(self.predicate)(&x) {
                    self.flag = true;
                }
                Some(x)
            })
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.iter.size_hint();
        (0, upper) // can't know a lower bound, due to the predicate
    }
}
*/
