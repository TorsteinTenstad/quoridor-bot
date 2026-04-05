#[derive(Clone)]
pub struct Buffer<T, const N: usize> {
    buf: [T; N],
    i: usize,
    len: usize,
}

impl<T: Default + Clone + Copy, const N: usize> Default for Buffer<T, N> {
    fn default() -> Self {
        Self {
            buf: [T::default(); N],
            i: 0,
            len: 0,
        }
    }
}

impl<T: Copy, const N: usize> Buffer<T, N> {
    pub fn insert(&mut self, e: T) {
        if self.len >= N {
            panic!("insert into full buffer");
        };

        let i = (self.i + self.len) % N;
        self.buf[i] = e;
        self.len += 1;
    }

    pub fn peek_first(&mut self) -> &T {
        if self.len <= 0 {
            panic!("peek empty buffer");
        };

        &self.buf[self.i]
    }

    pub fn pop_first(&mut self) -> T {
        if self.len <= 0 {
            panic!("pop empty buffer");
        };

        let e = self.buf[self.i];
        self.i = (self.i + 1) % N;
        self.len -= 1;
        e
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        (0..self.len).map(|i| &self.buf[(self.i + i) % N])
    }

    pub fn non_empty(&self) -> bool {
        self.len != 0
    }

    pub fn empty(&self) -> bool {
        self.len == 0
    }
}
