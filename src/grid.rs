use std::ops::{Index, IndexMut};

use nalgebra as na;

pub type Pos3 = na::Vector3<usize>;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Grid3<T> {
    size: Pos3,
    data: Vec<T>,
}

impl<T: Default + Copy> Grid3<T> {
    pub fn new(size: Pos3) -> Grid3<T> {
        Grid3 {
            size,
            data: vec![Default::default(); size.x * size.y * size.z],
        }
    }
}

impl<T> Grid3<T> {
    pub fn node_index(&self, p: Pos3) -> usize {
        p.x
        + p.y * self.size.x
        + p.z * self.size.x * self.size.y
    }
}

impl<T> Index<Pos3> for Grid3<T> {
    type Output = T;

    fn index<'a>(&'a self, p: Pos3) -> &'a T {
        &self.data[self.node_index(p)]
    }
}

impl<T> IndexMut<Pos3> for Grid3<T> {
    fn index_mut<'a>(&'a mut self, p: Pos3) -> &'a mut T {
        let index = self.node_index(p);
        &mut self.data[index]
    }
}
