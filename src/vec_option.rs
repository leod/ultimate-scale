use std::ops::{Index, IndexMut};
use std::collections::VecDeque;

pub struct VecOption<T> {
    data: Vec<Option<T>>,
    size: usize,
    free: VecDeque<usize>,
}

impl<T> VecOption<T> {
    pub fn new() -> VecOption<T> {
        VecOption {
            data: Vec::new(),
            size: 0,
            free: VecDeque::new(),
        }
    }

    pub fn add(&mut self, value: T) -> usize {
        self.size += 1;    

        if let Some(index) = self.free.pop_front() {
            debug_assert!(self.data[index].is_none());

            self.data[index] = Some(value);
            index
        } else {
            self.data.push(Some(value));
            self.data.len() - 1
        }
    }

    pub fn remove(&mut self, index: usize) -> Option<T> {
        let value = self.data[index].take();

        if value.is_some() {
            self.size -= 1;
            self.free.push_back(index);
        }

        value
    }

    pub fn iter(&self) -> Iter<T> {
        Iter {
            vec: self,
            i: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.size
    }

    fn next_used_index(&self, mut i: usize) -> Option<usize> {
        while i < self.data.len() && self.data[i].is_none() {
            i += 1;
        }

        if i < self.data.len() {
            debug_assert!(self.data[i].is_some());

            Some(i)
        } else {
            None
        }
    }
}

impl<T> Index<usize> for VecOption<T> {
    type Output = T;

    fn index<'a>(&'a self, index: usize) -> &'a T {
        self.data[index].as_ref().unwrap()
    }
}

impl<T> IndexMut<usize> for VecOption<T> {
    fn index_mut<'a>(&'a mut self, index: usize) -> &'a mut T {
        self.data[index].as_mut().unwrap()
    }
}

pub struct Iter<'a, T> {
    vec: &'a VecOption<T>,
    i: usize,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        if let Some(used_i) = self.vec.next_used_index(self.i) {
            self.i = used_i + 1;
            self.vec.data[used_i].as_ref()
        } else {
            None
        }
    }
}
