use std::ops::{Index, IndexMut};

use serde::{Deserialize, Serialize};

use nalgebra as na;

pub type Vector2 = na::Vector2<isize>;
pub type Vector3 = na::Vector3<isize>;
pub type Point3 = na::Point3<isize>;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum Axis2 {
    X,
    Y,
}

impl Axis2 {
    pub const NUM_INDICES: usize = 2;
    pub const ALL: [Axis2; Self::NUM_INDICES] = [Axis2::X, Axis2::Y];

    pub fn to_vector(self) -> Vector2 {
        match self {
            Axis2::X => Vector2::x(),
            Axis2::Y => Vector2::y(),
        }
    }

    pub fn to_index(self) -> usize {
        match self {
            Axis2::X => 0,
            Axis2::Y => 1,
        }
    }

    pub fn embed(self) -> Axis3 {
        match self {
            Axis2::X => Axis3::X,
            Axis2::Y => Axis3::Y,
        }
    }

    pub fn next(self) -> Axis2 {
        match self {
            Axis2::X => Axis2::Y,
            Axis2::Y => Axis2::X,
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum Axis3 {
    X,
    Y,
    Z,
}

impl Axis3 {
    pub const NUM_INDICES: usize = 3;
    pub const ALL: [Axis3; Self::NUM_INDICES] = [Axis3::X, Axis3::Y, Axis3::Z];

    pub fn to_vector(self) -> Vector3 {
        match self {
            Axis3::X => Vector3::x(),
            Axis3::Y => Vector3::y(),
            Axis3::Z => Vector3::z(),
        }
    }

    pub fn to_index(self) -> usize {
        match self {
            Axis3::X => 0,
            Axis3::Y => 1,
            Axis3::Z => 2,
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum Sign {
    Pos,
    Neg,
}

impl Sign {
    pub const NUM_INDICES: usize = 2;

    pub fn to_number(self) -> isize {
        match self {
            Sign::Pos => 1,
            Sign::Neg => -1,
        }
    }

    pub fn to_f32(self) -> f32 {
        self.to_number() as f32
    }

    pub fn invert(self) -> Sign {
        match self {
            Sign::Pos => Sign::Neg,
            Sign::Neg => Sign::Pos,
        }
    }

    pub fn to_index(self) -> usize {
        match self {
            Sign::Pos => 0,
            Sign::Neg => 1,
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Dir2(pub Axis2, pub Sign);

impl Dir2 {
    pub const NUM_INDICES: usize = Axis2::NUM_INDICES * Sign::NUM_INDICES;

    pub const X_POS: Dir2 = Dir2(Axis2::X, Sign::Pos);
    pub const X_NEG: Dir2 = Dir2(Axis2::X, Sign::Neg);
    pub const Y_POS: Dir2 = Dir2(Axis2::Y, Sign::Pos);
    pub const Y_NEG: Dir2 = Dir2(Axis2::Y, Sign::Neg);

    pub fn to_vector(self) -> Vector2 {
        self.0.to_vector() * self.1.to_number()
    }

    pub fn invert(self) -> Dir2 {
        Dir2(self.0, self.1.invert())
    }

    pub fn rotated_cw(self) -> Dir2 {
        let sign = match self.0 {
            Axis2::X => self.1.invert(),
            Axis2::Y => self.1,
        };

        Dir2(self.0.next(), sign)
    }

    pub fn rotated_ccw(self) -> Dir2 {
        let sign = match self.0 {
            Axis2::X => self.1,
            Axis2::Y => self.1.invert(),
        };

        Dir2(self.0.next(), sign)
    }

    pub fn to_radians(self) -> f32 {
        let vector = self.to_vector();
        (vector.y as f32).atan2(vector.x as f32)
    }

    pub fn to_index(self) -> usize {
        self.0.to_index() * Sign::NUM_INDICES + self.1.to_index()
    }

    pub fn embed(self) -> Dir3 {
        Dir3(self.0.embed(), self.1)
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Dir3(pub Axis3, pub Sign);

impl Dir3 {
    pub const NUM_INDICES: usize = Axis3::NUM_INDICES * Sign::NUM_INDICES;

    pub const X_POS: Dir3 = Dir3(Axis3::X, Sign::Pos);
    pub const X_NEG: Dir3 = Dir3(Axis3::X, Sign::Neg);
    pub const Y_POS: Dir3 = Dir3(Axis3::Y, Sign::Pos);
    pub const Y_NEG: Dir3 = Dir3(Axis3::Y, Sign::Neg);
    pub const Z_POS: Dir3 = Dir3(Axis3::Z, Sign::Pos);
    pub const Z_NEG: Dir3 = Dir3(Axis3::Z, Sign::Neg);

    pub const ALL: [Dir3; Self::NUM_INDICES] = [
        Dir3::X_POS,
        Dir3::X_NEG,
        Dir3::Y_POS,
        Dir3::Y_NEG,
        Dir3::Z_POS,
        Dir3::Z_NEG,
    ];

    pub fn to_vector(self) -> Vector3 {
        self.0.to_vector() * self.1.to_number()
    }

    pub fn invert(self) -> Dir3 {
        Dir3(self.0, self.1.invert())
    }

    pub fn to_index(self) -> usize {
        self.0.to_index() * Sign::NUM_INDICES + self.1.to_index()
    }

    pub fn rotated_cw_xy(self) -> Dir3 {
        let axis = match self.0 {
            Axis3::X => Axis3::Y,
            Axis3::Y => Axis3::X,
            Axis3::Z => Axis3::Z,
        };
        let sign = match self.0 {
            Axis3::X => self.1.invert(),
            Axis3::Y | Axis3::Z => self.1,
        };
        Dir3(axis, sign)
    }

    pub fn rotated_ccw_xy(self) -> Dir3 {
        let axis = match self.0 {
            Axis3::X => Axis3::Y,
            Axis3::Y => Axis3::X,
            Axis3::Z => Axis3::Z,
        };
        let sign = match self.0 {
            Axis3::Y => self.1.invert(),
            Axis3::X | Axis3::Z => self.1,
        };
        Dir3(axis, sign)
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Grid3<T> {
    size: Vector3,
    data: Vec<T>,
}

impl<T> Default for Grid3<T> {
    fn default() -> Self {
        Grid3 {
            size: Vector3::zeros(),
            data: Vec::new(),
        }
    }
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

        let index = p.x + p.y * self.size.x + p.z * self.size.x * self.size.y;

        index as usize
    }

    pub fn is_valid_pos(&self, p: &Point3) -> bool {
        p.x >= 0
            && p.x < self.size.x
            && p.y >= 0
            && p.y < self.size.y
            && p.z >= 0
            && p.z < self.size.z
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

    fn index(&self, p: Point3) -> &T {
        &self.data[self.node_index(&p)]
    }
}

impl<T> IndexMut<Point3> for Grid3<T> {
    fn index_mut(&mut self, p: Point3) -> &mut T {
        let index = self.node_index(&p);
        &mut self.data[index]
    }
}
