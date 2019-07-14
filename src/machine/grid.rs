use std::ops::{Index, IndexMut};

use nalgebra as na;

pub type Vector3 = na::Vector3<isize>;
pub type Point3 = na::Point3<isize>;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Grid3<T> {
    size: Vector3,
    data: Vec<T>,
}

impl<T: Default + Copy> Grid3<T> {
    pub fn new(size: Vector3) -> Grid3<T> {
        assert!(size.x >= 0 && size.y >= 0 && size.z >= 0);
        let n = (size.x * size.y * size.z) as usize;

        Grid3 {
            size,
            data: vec![Default::default(); n],
        }
    }
}

impl<T> Grid3<T> {
    pub fn node_index(&self, p: &Point3) -> usize {
        debug_assert!(self.is_valid_pos(p));

        let index =
            p.x +
            p.y * self.size.x +
            p.z * self.size.x * self.size.y;
        
        index as usize
    }

    pub fn is_valid_pos(&self, p: &Point3) -> bool {
        p.x >= 0 && p.x < self.size.x &&
        p.y >= 0 && p.y < self.size.y &&
        p.z >= 0 && p.z < self.size.z
    }

    pub fn get(&self, p: &Point3) -> Option<&T> {
        if self.is_valid_pos(p) {
            Some(&self[*p])
        } else {
            None
        }
    }

    pub fn size(&self) -> Vector3 {
        self.size
    }
}

impl<T> Index<Point3> for Grid3<T> {
    type Output = T;

    fn index<'a>(&'a self, p: Point3) -> &'a T {
        &self.data[self.node_index(&p)]
    }
}

impl<T> IndexMut<Point3> for Grid3<T> {
    fn index_mut<'a>(&'a mut self, p: Point3) -> &'a mut T {
        let index = self.node_index(&p);
        &mut self.data[index]
    }
}
