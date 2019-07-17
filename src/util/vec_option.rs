use std::collections::VecDeque;
use std::iter::Enumerate;
use std::ops::{Index, IndexMut};

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct VecOption<T> {
    data: Vec<Option<T>>,
    free: VecDeque<usize>,
    size: usize,
}

impl<T> VecOption<T> {
    pub fn new() -> VecOption<T> {
        VecOption {
            data: Vec::new(),
            free: VecDeque::new(),
            size: 0,
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
            iter: self.data.iter().enumerate(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<T> {
        IterMut {
            iter: self.data.iter_mut().enumerate(),
        }
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.free.clear();
        self.size = 0;
    }

    pub fn num_free(&self) -> usize {
        let num = self.free.len();

        debug_assert!(num == self.data.iter().filter(|x| x.is_none()).count());

        num
    }
}

impl<T: Clone> VecOption<T> {
    pub fn gc(&mut self) {
        self.data = self
            .data
            .iter()
            .filter(|x| x.is_some())
            .cloned()
            .collect();
        self.free.clear();
    }
}

impl<T> Index<usize> for VecOption<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        self.data[index].as_ref().unwrap()
    }
}

impl<T> IndexMut<usize> for VecOption<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        self.data[index].as_mut().unwrap()
    }
}

pub struct Iter<'a, T> {
    iter: Enumerate<std::slice::Iter<'a, Option<T>>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        let mut elem = self.iter.next();

        while {
            if let Some(slot) = elem {
                slot.1.is_none()
            } else {
                false
            }
        } {
            elem = self.iter.next();
        }

        if let Some(slot) = elem {
            slot.1.as_ref().map(|value| (slot.0, value))
        } else {
            None
        }
    }
}

pub struct IterMut<'a, T: 'a> {
    iter: Enumerate<std::slice::IterMut<'a, Option<T>>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (usize, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        let mut elem = self.iter.next();

        while {
            if let Some(slot) = &elem {
                slot.1.is_none()
            } else {
                false
            }
        } {
            elem = self.iter.next();
        }

        if let Some(slot) = elem {
            if let (index, Some(value)) = slot {
                Some((index, value))
            } else {
                None
            }
        } else {
            None
        }
    }
}
