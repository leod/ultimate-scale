use std::ops::Mul;

use crate::edit::Edit;
use crate::machine::grid;
use crate::machine::{Machine, PlacedBlock};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transform {
    Shift(grid::Vector3),
    RotateCWXY,
    RotateCCWXY,
    MirrorY,
    Seq(Vec<Transform>),
}

impl<'a> Mul<grid::Point3> for &'a Transform {
    type Output = grid::Point3;

    fn mul(self, p: grid::Point3) -> grid::Point3 {
        match self {
            Transform::Shift(delta) => p + delta,
            Transform::RotateCWXY => grid::Point3::new(p.y, -p.x, p.z),
            Transform::RotateCCWXY => grid::Point3::new(-p.y, p.x, p.z),
            Transform::MirrorY => grid::Point3::new(-p.x, p.y, p.z),
            Transform::Seq(inner) => inner.iter().fold(p, |p, transform| transform * p),
        }
    }
}

impl<'a> Mul<(isize, isize, isize)> for &'a Transform {
    type Output = grid::Point3;

    fn mul(self, p: (isize, isize, isize)) -> grid::Point3 {
        self * grid::Point3::new(p.0, p.1, p.2)
    }
}

impl<'a> Mul<grid::Dir3> for &'a Transform {
    type Output = grid::Dir3;

    fn mul(self, d: grid::Dir3) -> grid::Dir3 {
        match self {
            Transform::Shift(_) => d,
            Transform::RotateCWXY => d.rotated_cw_xy(),
            Transform::RotateCCWXY => d.rotated_ccw_xy(),
            Transform::MirrorY => d.mirrored_y(),
            Transform::Seq(inner) => inner.iter().fold(d, |d, transform| transform * d),
        }
    }
}

/// A piece of a machine that can be kept around as edit actions, or in the
/// clipboard and stuff like that.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Piece {
    blocks: Vec<(grid::Point3, PlacedBlock)>,
}

impl Piece {
    pub fn new_origin_block(block: PlacedBlock) -> Self {
        Self {
            blocks: vec![(grid::Point3::origin(), block)],
        }
    }

    pub fn new(blocks: Vec<(grid::Point3, PlacedBlock)>) -> Self {
        Piece { blocks }
    }

    pub fn new_from_selection(
        machine: &Machine,
        selection: impl Iterator<Item = grid::Point3>,
    ) -> Self {
        let blocks =
            selection.filter_map(|pos| machine.get(&pos).map(|block| (pos, block.clone())));

        Self::new(blocks.collect())
    }

    pub fn iter(&self) -> impl Iterator<Item = (grid::Point3, PlacedBlock)> + '_ {
        self.blocks.iter().map(|(pos, block)| (*pos, block.clone()))
    }

    #[allow(dead_code)]
    pub fn blocks(&self) -> &[(grid::Point3, PlacedBlock)] {
        &self.blocks
    }

    pub fn transform(&mut self, transform: &Transform) {
        for (pos, placed_block) in self.blocks.iter_mut() {
            *pos = transform * *pos;
            placed_block.block.mutate_dirs(|dir| transform * dir);
        }
    }

    pub fn shift(&mut self, delta: &grid::Vector3) {
        self.transform(&Transform::Shift(*delta));
    }

    pub fn rotate_cw_xy(&mut self) {
        self.transform(&Transform::RotateCWXY);
    }

    pub fn rotate_ccw_xy(&mut self) {
        self.transform(&Transform::RotateCCWXY);
    }

    pub fn mirror_y(&mut self) {
        self.transform(&Transform::MirrorY);
    }

    pub fn set_next_kind(&mut self) {
        for (_, placed_block) in self.blocks.iter_mut() {
            if let Some(kind) = placed_block.block.kind() {
                placed_block.block.set_kind(kind.next());
            }
        }
    }

    pub fn as_place_edit(&self) -> Edit {
        let set_blocks = self.iter().map(|(pos, block)| (pos, Some(block))).collect();

        Edit::SetBlocks(set_blocks)
    }

    pub fn get_singleton(&self) -> Option<(grid::Point3, PlacedBlock)> {
        if let Some(entry) = self.blocks.iter().next() {
            if self.blocks.len() == 1 {
                Some(entry.clone())
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn min_pos(&self) -> grid::Point3 {
        let mut min = grid::Point3::new(std::isize::MAX, std::isize::MAX, std::isize::MAX);

        for (p, _) in &self.blocks {
            if p.x < min.x {
                min.x = p.x;
            }
            if p.y < min.y {
                min.y = p.y;
            }
            if p.z < min.z {
                min.z = p.z;
            }
        }

        min
    }

    pub fn max_pos(&self) -> grid::Point3 {
        let mut max = grid::Point3::new(std::isize::MIN, std::isize::MIN, std::isize::MIN);

        for (p, _) in &self.blocks {
            if p.x > max.x {
                max.x = p.x;
            }
            if p.y > max.y {
                max.y = p.y;
            }
            if p.z > max.z {
                max.z = p.z;
            }
        }

        max
    }

    pub fn extent(&self) -> grid::Vector3 {
        self.max_pos() - self.min_pos() + grid::Vector3::new(1, 1, 1)
    }
}
