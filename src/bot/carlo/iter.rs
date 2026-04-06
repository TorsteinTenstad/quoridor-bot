pub enum ABIter<L, R> {
    A(L),
    B(R),
}

impl<A, B, T> Iterator for ABIter<A, B>
where
    A: Iterator<Item = T>,
    B: Iterator<Item = T>,
{
    type Item = T;

    fn next(&mut self) -> Option<T> {
        match self {
            ABIter::A(i) => i.next(),
            ABIter::B(i) => i.next(),
        }
    }
}

pub enum ABCIter<A, B, C> {
    A(A),
    B(B),
    C(C),
}

impl<A, B, C, T> Iterator for ABCIter<A, B, C>
where
    A: Iterator<Item = T>,
    B: Iterator<Item = T>,
    C: Iterator<Item = T>,
{
    type Item = T;

    fn next(&mut self) -> Option<T> {
        match self {
            ABCIter::A(i) => i.next(),
            ABCIter::B(i) => i.next(),
            ABCIter::C(i) => i.next(),
        }
    }
}
