use std::fmt::Debug;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Left,
    Right,
    Up,
    Down,
}

impl From<(i8, i8)> for Dir {
    fn from(dxdy: (i8, i8)) -> Self {
        match dxdy {
            (-1, _) => Dir::Left,
            (1, _) => Dir::Right,
            (_, -1) => Dir::Up,
            (_, 1) => Dir::Down,
            _ => unreachable!(),
        }
    }
}

impl Dir {
    pub fn reverse(&self) -> Dir {
        match self {
            Dir::Left => Dir::Right,
            Dir::Right => Dir::Left,
            Dir::Up => Dir::Down,
            Dir::Down => Dir::Up,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PathBlock {
    Unreachable,
    Goal,
    Dir(Dir),
}

impl Debug for PathBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreachable => write!(f, "⊗"),
            Self::Goal => write!(f, "⊙"),
            Self::Dir(Dir::Left) => write!(f, "🠈"),
            Self::Dir(Dir::Right) => write!(f, "🠊"),
            Self::Dir(Dir::Up) => write!(f, "🠉"),
            Self::Dir(Dir::Down) => write!(f, "🠋"),
        }
    }
}
