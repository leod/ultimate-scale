use crate::edit::Edit;
use crate::machine::grid;
use crate::machine::{Machine, PlacedBlock};

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
        let blocks = selection.filter_map(|pos| {
            machine
                .get_block_at_pos(&pos)
                .map(|(_, block)| (pos, block.clone()))
        });

        Self::new(blocks.collect())
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

    pub fn shift(&mut self, delta: &grid::Vector3) {
        for (pos, _) in self.blocks.iter_mut() {
            *pos += delta;
        }
    }

    pub fn rotate_cw_xy(&mut self) {
        for (pos, placed_block) in self.blocks.iter_mut() {
            *pos = grid::Point3::new(pos.y, -pos.x, pos.z);
            placed_block.block.mutate_dirs(grid::Dir3::rotated_cw_xy);
        }
    }

    pub fn rotate_ccw_xy(&mut self) {
        for _ in 0..3 {
            self.rotate_cw_xy();
        }
    }

    pub fn mirror_y(&mut self) {
        let max_pos = self.max_pos();

        for (pos, placed_block) in self.blocks.iter_mut() {
            *pos = grid::Point3::new(max_pos.x - pos.x - 1, pos.y, pos.z);

            placed_block.block.mutate_dirs(|dir| {
                if dir.0 == grid::Axis3::X {
                    dir.invert()
                } else {
                    dir
                }
            });
        }
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

    pub fn iter(&self) -> impl Iterator<Item = (grid::Point3, PlacedBlock)> + '_ {
        self.blocks.iter().map(|(pos, block)| (*pos, block.clone()))
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
}
