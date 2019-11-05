use crate::machine::grid;
use crate::machine::{Machine, PlacedBlock};
use crate::edit::Edit;

/// A piece of a machine that can be kept around as edit actions, or in the
/// clipboard and stuff like that.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Piece {
    /// Blocks that shall be placed. All point coordinates are assumed to be
    /// non-negative.
    blocks: Vec<(grid::Point3, PlacedBlock)>,
}

impl Piece {
    pub fn new_origin_block(block: PlacedBlock) -> Self {
        Self {
            blocks: vec![(grid::Point3::origin(), block)],
        }
    }

    pub fn new_blocks_to_origin(blocks: &[(grid::Point3, PlacedBlock)]) -> Self {
        Self {
            blocks: Self::blocks_to_origin(blocks),
        }
    }

    pub fn new_from_selection(
        machine: &Machine,
        selection: impl Iterator<Item = grid::Point3>,
    ) -> Self {
        Piece::new_blocks_to_origin(&Self::selected_blocks(machine, selection).collect::<Vec<_>>())
    }

    pub fn block_at_index(&self, index: usize) -> &(grid::Point3, PlacedBlock) {
        &self.blocks[index]
    }

    pub fn selected_blocks<'a>(
        machine: &'a Machine,
        selection: impl Iterator<Item = grid::Point3> + 'a,
    ) -> impl Iterator<Item = (grid::Point3, PlacedBlock)> + 'a {
        selection.filter_map(move |p| machine.get_block_at_pos(&p).map(|(_, b)| (p, b.clone())))
    }

    pub fn grid_size(&self) -> grid::Vector3 {
        let mut max = grid::Vector3::zeros();

        for (p, _) in self.blocks.iter() {
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

        max + grid::Vector3::new(1, 1, 1)
    }

    pub fn grid_center_xy(&self) -> grid::Vector3 {
        let size = self.grid_size();

        // Bias towards the origin for even sizes
        grid::Vector3::new(
            size.x / 2 - (size.x > 0 && size.x % 2 == 0) as isize,
            size.y / 2 - (size.y > 0 && size.y % 2 == 0) as isize,
            0,
        )
    }

    pub fn rotate_cw_xy(&mut self) {
        self.blocks = Self::blocks_to_origin(
            &self
                .blocks
                .clone()
                .into_iter()
                .map(|(p, mut placed_block)| {
                    let rotated_p = grid::Point3::new(p.y, self.grid_size().y - p.x, p.z);

                    placed_block.rotate_cw_xy();

                    (rotated_p, placed_block)
                })
                .collect::<Vec<_>>(),
        );
    }

    pub fn rotate_ccw_xy(&mut self) {
        for _ in 0..3 {
            self.rotate_cw_xy();
        }
    }

    pub fn next_kind(&mut self) {
        for (_, placed_block) in self.blocks.iter_mut() {
            if let Some(kind) = placed_block.block.kind() {
                placed_block.block = placed_block.block.with_kind(kind.next());
            }
        }
    }

    pub fn place_edit(&self, offset: &grid::Vector3) -> Edit {
        let set_blocks = self
            .iter_blocks(offset)
            .map(|(pos, block)| (pos, Some(block)))
            .collect();

        Edit::SetBlocks(set_blocks)
    }

    pub fn iter_blocks(
        &self,
        offset: &grid::Vector3,
    ) -> impl Iterator<Item = (grid::Point3, PlacedBlock)> + '_ {
        let offset = *offset;
        self.blocks
            .iter()
            .map(move |(pos, block)| (pos + offset, block.clone()))
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

    pub fn blocks_min_pos(blocks: &[(grid::Point3, PlacedBlock)]) -> grid::Point3 {
        let mut min = grid::Point3::new(std::isize::MAX, std::isize::MAX, std::isize::MAX);

        for (p, _) in blocks {
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

    pub fn blocks_to_origin(
        blocks: &[(grid::Point3, PlacedBlock)],
    ) -> Vec<(grid::Point3, PlacedBlock)> {
        let min = Self::blocks_min_pos(blocks);

        blocks
            .iter()
            .map(|(p, block)| (p - min.coords, block.clone()))
            .collect()
    }
}

