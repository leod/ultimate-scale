pub mod grid;

use crate::util::vec_option::VecOption;

use grid::{Axis3, Dir3, Grid3, Point3, Sign, Vector3};

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum BlipKind {
    A,
    B,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum Block {
    PipeXY,
    PipeBendXY,
    PipeSplitXY,
    WindSource,
    BlipSpawn(BlipKind),
    Solid,
}

impl Block {
    pub fn has_wind_hole(&self, dir: Dir3) -> bool {
        match self {
            Block::PipeXY => dir == Dir3(Axis3::Y, Sign::Neg) || dir == Dir3(Axis3::Y, Sign::Pos),
            Block::PipeBendXY => {
                dir == Dir3(Axis3::X, Sign::Neg) || dir == Dir3(Axis3::Y, Sign::Pos)
            }
            Block::PipeSplitXY => {
                dir == Dir3(Axis3::Y, Sign::Neg)
                    || dir == Dir3(Axis3::Y, Sign::Pos)
                    || dir == Dir3(Axis3::X, Sign::Pos)
            }
            Block::WindSource => true,
            Block::BlipSpawn(_kind) => false,
            Block::Solid => false,
        }
    }

    pub fn allows_flow(&self) -> bool {
        match self {
            Block::Solid => false,
            _ => true,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct PlacedBlock {
    pub rotation_xy: usize,
    pub block: Block,
}

impl PlacedBlock {
    pub fn rotate_cw(&mut self) {
        self.rotation_xy += 1;
        if self.rotation_xy == 4 {
            self.rotation_xy = 0;
        }
    }

    pub fn rotate_ccw(&mut self) {
        if self.rotation_xy == 0 {
            self.rotation_xy = 3;
        } else {
            self.rotation_xy -= 1;
        }
    }

    pub fn rotated_dir_xy(&self, mut dir: Dir3) -> Dir3 {
        for _ in 0..self.rotation_xy {
            dir = dir.rotated_cw_xy();
        }

        dir
    }

    pub fn rotated_dir_ccw_xy(&self, mut dir: Dir3) -> Dir3 {
        for _ in 0..self.rotation_xy {
            dir = dir.rotated_ccw_xy();
        }

        dir
    }

    pub fn angle_xy_radians(&self) -> f32 {
        -std::f32::consts::PI / 2.0 * self.rotation_xy as f32
    }

    pub fn has_wind_hole(&self, dir: Dir3) -> bool {
        self.block.has_wind_hole(self.rotated_dir_ccw_xy(dir))
    }

    pub fn wind_holes(&self) -> Vec<Dir3> {
        // TODO: This could return an iterator to simplify optimizations
        // (or we could use generators, but they don't seem to be stable yet).

        (&Dir3::ALL)
            .iter()
            .filter(|dir| self.has_wind_hole(**dir))
            .copied()
            .collect()
    }
}

pub type BlockIndex = usize;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Machine {
    pub block_ids: Grid3<Option<BlockIndex>>,
    pub block_data: VecOption<(Point3, PlacedBlock)>,
}

impl Machine {
    pub fn empty() -> Machine {
        Machine {
            block_ids: Grid3::new(Vector3::new(0, 0, 0)),
            block_data: VecOption::new(),
        }
    }

    pub fn new(size: Vector3) -> Machine {
        Machine {
            block_ids: Grid3::new(size),
            block_data: VecOption::new(),
        }
    }

    pub fn size(&self) -> Vector3 {
        self.block_ids.size()
    }

    pub fn is_valid_pos(&self, p: &Point3) -> bool {
        self.block_ids.is_valid_pos(p)
    }

    pub fn is_valid_layer(&self, layer: isize) -> bool {
        layer >= 0 && layer < self.size().z
    }

    pub fn get_block_at_pos(&self, p: &Point3) -> Option<(BlockIndex, &PlacedBlock)> {
        self.block_ids
            .get(p)
            .and_then(|id| id.as_ref())
            .map(|&id| (id, &self.block_data[id].1))
    }

    pub fn block_at_index(&self, index: BlockIndex) -> &(Point3, PlacedBlock) {
        &self.block_data[index]
    }

    pub fn set_block_at_pos(&mut self, p: &Point3, block: Option<PlacedBlock>) {
        self.remove_at_pos(p);

        if let Some(block) = block {
            let id = self.block_data.add((*p, block));
            self.block_ids[*p] = Some(id);
        }
    }

    pub fn remove_at_pos(&mut self, p: &Point3) -> Option<(BlockIndex, PlacedBlock)> {
        if let Some(Some(id)) = self.block_ids.get(p).cloned() {
            self.block_ids[*p] = None;
            self.block_data.remove(id).map(|(data_pos, block)| {
                assert!(data_pos == *p);
                (id, block)
            })
        } else {
            None
        }
    }

    pub fn iter_blocks(&self) -> impl Iterator<Item = (usize, &(Point3, PlacedBlock))> {
        self.block_data.iter()
    }

    pub fn iter_blocks_mut(&mut self) -> impl Iterator<Item = (usize, &mut (Point3, PlacedBlock))> {
        self.block_data.iter_mut()
    }

    pub fn gc(&mut self) {
        self.block_data.gc();

        for (index, (grid_pos, _)) in self.block_data.iter() {
            self.block_ids[*grid_pos] = Some(index);
        }
    }

    pub fn is_contiguous(&self) -> bool {
        self.block_data.num_free() == 0
    }

    pub fn num_blocks(&self) -> usize {
        self.block_data.len()
    }
}
