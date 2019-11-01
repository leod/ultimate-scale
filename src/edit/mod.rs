pub mod config;
pub mod editor;
pub mod pick;

use std::collections::HashMap;

use nalgebra as na;

use crate::machine::grid;
use crate::machine::{Machine, PlacedBlock};

pub use config::Config;
pub use editor::Editor;

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

#[derive(Debug, Clone)]
pub enum Edit {
    NoOp,
    SetBlocks(HashMap<grid::Point3, Option<PlacedBlock>>),

    /// Rotate blocks clockwise.
    RotateCWXY(Vec<grid::Point3>),

    /// Rotate blocks counterclockwise.
    RotateCCWXY(Vec<grid::Point3>),

    /// Run two edits in sequence.
    Pair(Box<Edit>, Box<Edit>),
}

impl Edit {
    /// Apply the edit operation to a machine and return an edit operation to
    /// undo what was done.
    pub fn run(self, machine: &mut Machine) -> Edit {
        match self {
            Edit::NoOp => Edit::NoOp,
            Edit::SetBlocks(blocks) => {
                let valid_blocks = blocks
                    .into_iter()
                    .filter(|(p, _block)| machine.is_valid_pos(p))
                    .collect::<HashMap<_, _>>();

                let previous_blocks = valid_blocks
                    .keys()
                    .map(|p| {
                        (
                            *p,
                            machine
                                .get_block_at_pos(p)
                                .map(|(_index, block)| block.clone()),
                        )
                    })
                    .collect();

                if previous_blocks == valid_blocks {
                    Edit::NoOp
                } else {
                    for (p, block) in valid_blocks.iter() {
                        machine.set_block_at_pos(p, block.clone());
                    }

                    Edit::SetBlocks(previous_blocks)
                }
            }
            Edit::RotateCWXY(points) => {
                for p in &points {
                    if let Some((_, placed_block)) = machine.get_block_at_pos_mut(p) {
                        placed_block.rotate_cw_xy();
                    }
                }

                if points.is_empty() {
                    Edit::NoOp
                } else {
                    Edit::RotateCCWXY(points)
                }
            }
            Edit::RotateCCWXY(points) => {
                for p in &points {
                    if let Some((_, placed_block)) = machine.get_block_at_pos_mut(p) {
                        placed_block.rotate_ccw_xy();
                    }
                }

                if points.is_empty() {
                    Edit::NoOp
                } else {
                    Edit::RotateCWXY(points)
                }
            }
            Edit::Pair(a, b) => {
                let undo_a = a.run(machine);
                let undo_b = b.run(machine);

                Self::compose(undo_b, undo_a)
            }
        }
    }

    pub fn compose(a: Edit, b: Edit) -> Edit {
        match (a, b) {
            (Edit::NoOp, Edit::NoOp) => Edit::NoOp,
            (Edit::SetBlocks(mut a), Edit::SetBlocks(b)) => {
                for (p, block) in b.into_iter() {
                    a.insert(p, block);
                }

                Edit::SetBlocks(a)
            }
            (a, b) => Edit::Pair(Box::new(a), Box::new(b)),
        }
    }
}

/// Modes that the editor can be in.
#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    /// Select blocks in the machine.
    ///
    /// For consistency, the selected positions must always contain a block.
    /// There must be no duplicate positions. The order corresponds to the
    /// selection order.
    Select(Vec<grid::Point3>),

    /// Select blocks in the machine by a screen rectangle.
    RectSelect {
        /// Blocks that were already selected when entering this mode.
        existing_selection: Vec<grid::Point3>,

        /// New blocks currently selected by the rectangle.
        new_selection: Vec<grid::Point3>,

        /// Start position of the rectangle.
        start_pos: na::Point2<f32>,

        /// Current end position of the rectangle.
        end_pos: na::Point2<f32>,
    },

    PlacePiece {
        piece: Piece,
        offset: grid::Vector3,
    },

    DragAndDrop {
        /// Selection that is being dragged. No duplicate positions, and each
        /// must contain a block in the machine.
        selection: Vec<grid::Point3>,

        /// Position that is being dragged, i.e. the block that was grabbed by
        /// the user.
        center_pos: grid::Point3,

        /// Rotation to be applied to the piece.
        rotation_xy: usize,

        layer_offset: isize,
    },
}

impl Mode {
    /// Make sure the mode state is consistent with the edited machine.
    ///
    /// The main case is for this to be called after an edit has been applied to
    /// the machine. In that case, the edit may have cleared out a block
    /// position which is currently selected in a `Mode`, so we need to remove
    /// it from the selection.
    pub fn make_consistent_with_machine(self, machine: &Machine) -> Self {
        match self {
            Mode::Select(mut selection) => {
                selection.retain(|grid_pos| machine.get_block_at_pos(grid_pos).is_some());
                Mode::Select(selection)
            }
            Mode::RectSelect {
                mut existing_selection,
                mut new_selection,
                start_pos,
                end_pos,
            } => {
                existing_selection.retain(|grid_pos| machine.get_block_at_pos(grid_pos).is_some());
                new_selection.retain(|grid_pos| machine.get_block_at_pos(grid_pos).is_some());
                Mode::RectSelect {
                    existing_selection,
                    new_selection,
                    start_pos,
                    end_pos,
                }
            }
            Mode::DragAndDrop {
                mut selection,
                center_pos,
                rotation_xy,
                layer_offset,
            } => {
                selection.retain(|grid_pos| machine.get_block_at_pos(grid_pos).is_some());

                if !selection.contains(&center_pos) {
                    // If the center block is not selected anymore, let's just
                    // not bother with this.
                    Mode::Select(selection)
                } else {
                    Mode::DragAndDrop {
                        selection,
                        center_pos,
                        rotation_xy,
                        layer_offset,
                    }
                }
            }
            mode => mode,
        }
    }
}
