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
            iter: self.data.iter(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<T> {
        IterMut {
            iter: self.data.iter_mut(),
        }
    }

    pub fn len(&self) -> usize {
        self.size
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
    iter: std::slice::Iter<'a, Option<T>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        let mut value = self.iter.next();

        while {
            if let Some(inner) = &value {
                inner.is_none()
            } else {
                false
            }
        } {
            value = self.iter.next();
        }

        if let Some(inner) = value {
            inner.as_ref()
        } else {
            None
        }
    }
}

pub struct IterMut<'a, T: 'a> {
    iter: std::slice::IterMut<'a, Option<T>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<&'a mut T> {
        let mut value = self.iter.next();

        while {
            if let Some(inner) = &value {
                inner.is_none()
            } else {
                false
            }
        } {
            value = self.iter.next();
        }

        if let Some(inner) = value {
            inner.as_mut()
        } else {
            None
        }
    }
}
