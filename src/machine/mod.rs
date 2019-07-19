pub mod grid;

use serde::ser::{SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};

use crate::util::vec_option::VecOption;

use grid::{Axis3, Dir3, Grid3, Point3, Sign, Vector3};

#[derive(PartialEq, Eq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum BlipKind {
    A,
    B,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum Block {
    PipeXY,
    PipeBendXY,
    PipeZ,
    PipeBendZ {
        sign_z: Sign,
    },
    PipeSplitXY {
        open_move_hole_y: Sign,
    },
    WindSource,
    BlipSpawn {
        kind: BlipKind,
        num_spawns: Option<usize>,
    },
    BlipDuplicator {
        activated: Option<BlipKind>,
    },
    BlipWindSource {
        activated: bool,
    },
    Solid,
}

impl Block {
    pub fn has_wind_hole(&self, dir: Dir3) -> bool {
        match self {
            Block::PipeXY => dir == Dir3::Y_NEG || dir == Dir3::Y_POS,
            Block::PipeBendXY => dir == Dir3::X_POS || dir == Dir3::Y_NEG,
            Block::PipeZ => dir == Dir3::Z_NEG || dir == Dir3::Z_POS,
            Block::PipeBendZ { sign_z } => dir == Dir3::Y_NEG || dir == Dir3(Axis3::Z, *sign_z),
            Block::PipeSplitXY { .. } => {
                dir == Dir3::Y_NEG || dir == Dir3::Y_POS || dir == Dir3::X_POS
            }
            Block::WindSource => true,
            Block::BlipSpawn { .. } => false,
            Block::BlipDuplicator { .. } => false,
            Block::Solid => false,
            Block::BlipWindSource { .. } => dir != Dir3::Y_NEG,
        }
    }

    pub fn has_move_hole(&self, dir: Dir3) -> bool {
        match self {
            Block::PipeSplitXY { open_move_hole_y } => {
                dir == Dir3(Axis3::Y, *open_move_hole_y) || dir == Dir3::X_POS
            }
            Block::BlipDuplicator { .. } => dir != Dir3::X_NEG || dir != Dir3::X_POS,
            Block::BlipWindSource { .. } => dir == Dir3::Y_NEG,
            _ => self.has_wind_hole(dir),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
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

    pub fn has_move_hole(&self, dir: Dir3) -> bool {
        self.block.has_move_hole(self.rotated_dir_ccw_xy(dir))
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
pub struct Blocks {
    pub indices: Grid3<Option<BlockIndex>>,
    pub data: VecOption<(Point3, PlacedBlock)>,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Machine {
    pub blocks: Blocks,
}

impl Machine {
    pub fn from_block_data(size: &Vector3, slice: &[(Point3, PlacedBlock)]) -> Self {
        let mut indices = Grid3::new(*size);
        let mut data = VecOption::new();

        for (pos, placed_block) in slice {
            indices[*pos] = Some(data.add((*pos, placed_block.clone())));
        }

        let blocks = Blocks { indices, data };

        Machine { blocks }
    }

    pub fn empty() -> Self {
        Self {
            blocks: Blocks {
                indices: Grid3::new(Vector3::new(0, 0, 0)),
                data: VecOption::new(),
            },
        }
    }

    pub fn new(size: Vector3) -> Self {
        Self {
            blocks: Blocks {
                indices: Grid3::new(size),
                data: VecOption::new(),
            },
        }
    }

    pub fn size(&self) -> Vector3 {
        self.blocks.indices.size()
    }

    pub fn is_valid_pos(&self, p: &Point3) -> bool {
        self.blocks.indices.is_valid_pos(p)
    }

    pub fn is_valid_layer(&self, layer: isize) -> bool {
        layer >= 0 && layer < self.size().z
    }

    pub fn get_block_at_pos(&self, p: &Point3) -> Option<(BlockIndex, &PlacedBlock)> {
        self.blocks
            .indices
            .get(p)
            .and_then(|id| id.as_ref())
            .map(|&id| (id, &self.blocks.data[id].1))
    }

    pub fn block_at_index(&self, index: BlockIndex) -> &(Point3, PlacedBlock) {
        &self.blocks.data[index]
    }

    pub fn set_block_at_pos(&mut self, p: &Point3, block: Option<PlacedBlock>) {
        self.remove_at_pos(p);

        if let Some(block) = block {
            let id = self.blocks.data.add((*p, block));
            self.blocks.indices[*p] = Some(id);
        }
    }

    pub fn remove_at_pos(&mut self, p: &Point3) -> Option<(BlockIndex, PlacedBlock)> {
        if let Some(Some(id)) = self.blocks.indices.get(p).cloned() {
            self.blocks.indices[*p] = None;
            self.blocks.data.remove(id).map(|(data_pos, block)| {
                assert!(data_pos == *p);
                (id, block)
            })
        } else {
            None
        }
    }

    pub fn iter_blocks(&self) -> impl Iterator<Item = (BlockIndex, &(Point3, PlacedBlock))> {
        self.blocks.data.iter()
    }

    pub fn iter_blocks_mut(
        &mut self,
    ) -> impl Iterator<Item = (BlockIndex, &mut (Point3, PlacedBlock))> {
        self.blocks.data.iter_mut()
    }

    pub fn gc(&mut self) {
        self.blocks.data.gc();

        for (index, (grid_pos, _)) in self.blocks.data.iter() {
            self.blocks.indices[*grid_pos] = Some(index);
        }
    }

    pub fn is_contiguous(&self) -> bool {
        self.blocks.data.num_free() == 0
    }

    pub fn num_blocks(&self) -> usize {
        self.blocks.data.len()
    }

    pub fn iter_neighbors<'a>(
        &'a self,
        pos: Point3,
    ) -> impl Iterator<Item = (Dir3, BlockIndex)> + 'a {
        Dir3::ALL.iter().filter_map(move |dir| {
            self.blocks
                .indices
                .get(&(pos + dir.to_vector()))
                .and_then(|index| index.as_ref())
                .map(|index| (*dir, *index))
        })
    }
}

/// Stores only the data necessary for restoring a machine.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SavedMachine {
    pub size: Vector3,
    pub block_data: Vec<(Point3, PlacedBlock)>,
}

impl SavedMachine {
    pub fn from_machine(machine: &Machine) -> Self {
        let block_data = machine
            .blocks
            .data
            .iter()
            .map(|(_index, data)| data.clone())
            .collect();

        Self {
            size: machine.size(),
            block_data,
        }
    }

    pub fn into_machine(&self) -> Machine {
        Machine::from_block_data(&self.size, &self.block_data)
    }
}
