use std::ops::{Index, IndexMut};

use nalgebra as na;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Grid3<T> {
    size: na::Vector3<usize>,
    data: Vec<T>,
}

impl<T: Default + Copy> Grid3<T> {
    pub fn new(size: na::Vector3<usize>) -> Grid3<T> {
        Grid3 {
            size,
            data: vec![Default::default(); size.x * size.y * size.z],
        }
    }
}

impl<T> Grid3<T> {
    pub fn node_index(&self, p: na::Vector3<usize>) -> usize {
        p.x
        + p.y * self.size.x
        + p.z * self.size.x * self.size.y
    }
}

impl<T> Index<na::Vector3<usize>> for Grid3<T> {
    type Output = T;

    fn index<'a>(&'a self, p: na::Vector3<usize>) -> &'a T {
        &self.data[self.node_index(p)]
    }
}

impl<T> IndexMut<na::Vector3<usize>> for Grid3<T> {
    fn index_mut<'a>(&'a mut self, p: na::Vector3<usize>) -> &'a mut T {
        let index = self.node_index(p);
        &mut self.data[index]
    }
}
