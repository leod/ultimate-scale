use std::ops::{Index, IndexMut};

use serde::{Deserialize, Serialize};

use nalgebra as na;

pub type Vector3 = na::Vector3<isize>;
pub type Point3 = na::Point3<isize>;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum Axis3 {
    X,
    Y,
    Z,
}

impl Axis3 {
    pub const NUM_INDICES: usize = 3;
    #[allow(dead_code)]
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

    pub const ALL_XY: [Dir3; 4] = [
        Dir3::X_POS,
        Dir3::X_NEG,
        Dir3::Y_POS,
        Dir3::Y_NEG,
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

    pub fn mirrored_y(self) -> Dir3 {
        if self.0 == Axis3::X {
            self.invert()
        } else {
            self
        }
    }

    /// Returns pitch and yaw to rotate an object that is oriented towards the x
    /// axis to point in our direction.
    ///
    /// See also:
    ///     https://raw.githubusercontent.com/limbusdev/images_for_wikimedia_commons/master/images/en/roll_pitch_yaw_en.png
    pub fn to_pitch_yaw_x(self) -> (f32, f32) {
        match self {
            Dir3(Axis3::X, Sign::Pos) => (0.0, 0.0),
            Dir3(Axis3::X, Sign::Neg) => (0.0, std::f32::consts::PI),
            Dir3(Axis3::Y, Sign::Pos) => (0.0, std::f32::consts::PI / 2.0),
            Dir3(Axis3::Y, Sign::Neg) => (0.0, 3.0 / 2.0 * std::f32::consts::PI),
            Dir3(Axis3::Z, Sign::Pos) => (-std::f32::consts::PI / 2.0, 0.0),
            Dir3(Axis3::Z, Sign::Neg) => (std::f32::consts::PI / 2.0, 0.0),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Default)]
pub struct DirMap3<T>(pub [T; Dir3::NUM_INDICES]);

impl<T> DirMap3<T> {
    pub fn from_fn(f: impl Fn(Dir3) -> T) -> Self {
        Self([
            f(Dir3::ALL[0]),    
            f(Dir3::ALL[1]),
            f(Dir3::ALL[2]),    
            f(Dir3::ALL[3]),    
            f(Dir3::ALL[4]),    
            f(Dir3::ALL[5]),    
        ])
    }

    pub fn keys(&self) -> impl Iterator<Item = Dir3> {
        Dir3::ALL.iter().cloned()
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.0.iter()
    }

    pub fn iter(&self) -> impl Iterator<Item = (Dir3, &T)> {
        self.keys().zip(self.values())
    }

    pub fn map<U>(self, f: impl Fn(Dir3, T) -> U) -> DirMap3<U> {
        Self([
            f(Dir3::ALL[0], self.0[0]),
            f(Dir3::ALL[1], self.0[1]),
            f(Dir3::ALL[2], self.0[2]),
            f(Dir3::ALL[3], self.0[3]),
            f(Dir3::ALL[4], self.0[4]),
            f(Dir3::ALL[5], self.0[5]),
        ])
    }
}

impl<T> Index<Dir3> for DirMap3<T> {
    type Output = T;

    fn index(&self, dir: Dir3) -> &T {
        &self.0[dir.to_index()]
    }
}

impl<T> IndexMut<Dir3> for DirMap3<T> {
    fn index_mut(&mut self, dir: Dir3) -> &mut T {
        &mut self.0[dir.to_index()]
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
