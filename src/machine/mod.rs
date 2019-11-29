pub mod grid;
pub mod level;
pub mod render;

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::util::vec_option::VecOption;

use grid::{Axis3, Dir3, Grid3, Point3, Sign, Vector3};
use level::Level;

#[derive(PartialEq, Eq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum BlipKind {
    A,
    B,
}

impl Default for BlipKind {
    fn default() -> BlipKind {
        BlipKind::A
    }
}

impl BlipKind {
    pub fn next(self) -> BlipKind {
        match self {
            BlipKind::A => BlipKind::B,
            BlipKind::B => BlipKind::A,
        }
    }
}

impl fmt::Display for BlipKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: These names are preliminary, should look into something that
        // avoids using colors.
        f.write_str(match self {
            BlipKind::A => "blue",
            BlipKind::B => "green",
        })
    }
}

pub type TickNum = usize;

/// Definition of a block in the machine.
///
/// This definition is somewhat "dirty" in that it also contains state that is
/// only needed at execution time -- e.g. the `activated` fields in some of the
/// blocks. Consider this an artifact of us not using an ECS.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum Block {
    Pipe(Dir3, Dir3),
    PipeMergeXY,
    FunnelXY {
        flow_dir: Dir3,
    },
    WindSource,
    BlipSpawn {
        out_dir: Dir3,
        kind: BlipKind,
        num_spawns: Option<usize>,

        #[serde(skip)]
        activated: Option<TickNum>,
    },
    BlipDuplicator {
        out_dirs: (Dir3, Dir3),
        kind: Option<BlipKind>,

        #[serde(skip)]
        activated: Option<BlipKind>,
    },
    BlipWindSource {
        button_dir: Dir3,

        #[serde(skip)]
        activated: bool,
    },
    Solid,
    Input {
        out_dir: Dir3,
        index: usize,

        #[serde(skip)]
        inputs: Vec<Option<level::Input>>,
        #[serde(skip)]
        activated: Option<level::Input>,
    },
    Output {
        in_dir: Dir3,
        index: usize,

        #[serde(skip)]
        outputs: Vec<BlipKind>,
        #[serde(skip)]
        activated: Option<BlipKind>,

        /// Only for visualization, store if this output failed.
        #[serde(skip)]
        failed: bool,
    },
    DetectorBlipDuplicator {
        out_dir: Dir3,
        flow_axis: Axis3,
        kind: Option<BlipKind>,

        #[serde(skip)]
        activated: Option<BlipKind>,
    },
}

impl Block {
    pub fn name(&self) -> String {
        match self {
            Block::Pipe(a, b) if a.0 != Axis3::Z && a.0 == b.0 => "Pipe straight".to_string(),
            Block::Pipe(a, b) if a.0 != Axis3::Z && b.0 != Axis3::Z && a.0 != b.0 => {
                "Pipe curve".to_string()
            }
            Block::Pipe(a, b) if a.0 == Axis3::Z && a.0 == b.0 => "Pipe up/down".to_string(),
            Block::Pipe(a, b) if (*a == Dir3::Z_NEG || *b == Dir3::Z_NEG) && a.0 != b.0 => {
                "Pipe curve down".to_string()
            }
            Block::Pipe(a, b) if (*a == Dir3::Z_POS || *b == Dir3::Z_POS) && a.0 != b.0 => {
                "Pipe curve up".to_string()
            }
            Block::Pipe(_, _) => "Pipe".to_string(),
            Block::PipeMergeXY => "Pipe crossing".to_string(),
            Block::FunnelXY { .. } => "Funnel".to_string(),
            Block::WindSource => "Wind source".to_string(),
            Block::BlipSpawn {
                num_spawns: None, ..
            } => "Blip source".to_string(),
            Block::BlipSpawn {
                num_spawns: Some(_),
                ..
            } => "Blip spawn".to_string(),
            Block::BlipDuplicator { kind: Some(_), .. } => "Picky blip copier".to_string(),
            Block::BlipDuplicator { kind: None, .. } => "Blip copier".to_string(),
            Block::BlipWindSource { .. } => "Blipped wind spawn".to_string(),
            Block::Solid => "Solid".to_string(),
            Block::Input { .. } => "Input".to_string(),
            Block::Output { .. } => "Output".to_string(),
            Block::DetectorBlipDuplicator { kind: Some(_), .. } => {
                "Picky detector blip copier".to_string()
            }
            Block::DetectorBlipDuplicator { kind: None, .. } => "Detector blip copier".to_string(),
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Block::Pipe(_, _) => "Conducts both wind and blips.",
            Block::PipeMergeXY => "Four-way pipe.",
            Block::FunnelXY { .. } => "Conducts in only one direction.",
            Block::WindSource => "Produces a stream of wind in all directions.",
            Block::BlipSpawn {
                num_spawns: None, ..
            } => "Produces a stream of blips.",
            Block::BlipSpawn {
                num_spawns: Some(1),
                ..
            } => "Spawns one blip.",
            Block::BlipSpawn {
                num_spawns: Some(_),
                ..
            } => "Spawns a limited number of blips.",
            Block::BlipDuplicator { kind: None, .. } => {
                "Produces two copies of whatever blip activates it.\n\nDESTROYS blips that are in its way!"
            }
            Block::BlipDuplicator { kind: Some(_), .. } => {
                "Produces two copies of a specific kind of blip that may activate it.\n\nDESTROYS blips that are in its way!"
            }
            Block::BlipWindSource { .. } => "Spawns one thrust of wind when activated by a blip.",
            Block::Solid => "Eats blips.",
            Block::Input { .. } => "Input of the machine.",
            Block::Output { .. } => "Output of the machine.",
            Block::DetectorBlipDuplicator { .. } => "TODO.",
        }
    }

    pub fn is_pipe(&self) -> bool {
        match self {
            Block::Pipe(_, _) => true,
            Block::PipeMergeXY => true,
            _ => false,
        }
    }

    pub fn kind(&self) -> Option<BlipKind> {
        match self {
            Block::BlipSpawn { kind, .. } => Some(*kind),
            Block::BlipDuplicator { kind, .. } => *kind,
            Block::DetectorBlipDuplicator { kind, .. } => *kind,
            _ => None,
        }
    }

    pub fn set_kind(&mut self, new_kind: BlipKind) {
        match self {
            Block::BlipSpawn { ref mut kind, .. } => *kind = new_kind,
            Block::BlipDuplicator { ref mut kind, .. } => *kind = Some(new_kind),
            Block::DetectorBlipDuplicator { ref mut kind, .. } => *kind = Some(new_kind),
            _ => (),
        }
    }

    pub fn mutate_dirs(&mut self, f: impl Fn(Dir3) -> Dir3) {
        match self {
            Block::Pipe(dir_a, dir_b) => {
                *dir_a = f(*dir_a);
                *dir_b = f(*dir_b);
            }
            Block::PipeMergeXY => (),
            Block::FunnelXY { flow_dir, .. } => *flow_dir = f(*flow_dir),
            Block::WindSource { .. } => (),
            Block::BlipSpawn { out_dir, .. } => *out_dir = f(*out_dir),
            Block::BlipDuplicator { out_dirs, .. } => {
                out_dirs.0 = f(out_dirs.0);
                out_dirs.1 = f(out_dirs.1);
            }
            Block::BlipWindSource { button_dir, .. } => *button_dir = f(*button_dir),
            Block::Solid => (),
            Block::Input { out_dir, .. } => *out_dir = f(*out_dir),
            Block::Output { in_dir, .. } => *in_dir = f(*in_dir),
            Block::DetectorBlipDuplicator {
                out_dir, flow_axis, ..
            } => {
                *out_dir = f(*out_dir);

                // Hack
                *flow_axis = f(Dir3(*flow_axis, Sign::Pos)).0;
            }
        }
    }

    pub fn has_wind_hole(&self, dir: Dir3) -> bool {
        match self {
            Block::Pipe(dir_a, dir_b) => dir == *dir_a || dir == *dir_b,
            Block::PipeMergeXY => dir != Dir3::Z_NEG && dir != Dir3::Z_POS,
            Block::FunnelXY { flow_dir, .. } => {
                // Has restricted cases for in/out below
                dir == *flow_dir || dir == flow_dir.invert()
            }
            Block::WindSource => true,
            Block::BlipSpawn { .. } => false,
            Block::BlipDuplicator { out_dirs, .. } => dir != out_dirs.0 && dir != out_dirs.1,
            Block::Solid => true,
            Block::BlipWindSource { .. } => true,
            Block::Input { out_dir, .. } => dir == *out_dir,
            Block::Output { in_dir, .. } => dir == *in_dir,
            Block::DetectorBlipDuplicator {
                out_dir, flow_axis, ..
            } => dir.0 == *flow_axis || dir == *out_dir,
        }
    }

    pub fn has_wind_hole_in(&self, dir: Dir3) -> bool {
        match self {
            Block::FunnelXY { flow_dir, .. } => dir == *flow_dir,
            Block::WindSource => false,
            Block::DetectorBlipDuplicator { flow_axis, .. } => dir.0 == *flow_axis,
            _ => self.has_wind_hole(dir),
        }
    }

    pub fn has_wind_hole_out(&self, dir: Dir3) -> bool {
        match self {
            Block::FunnelXY { flow_dir } => dir == flow_dir.invert(),
            Block::BlipDuplicator { .. } => false,
            Block::BlipWindSource { button_dir, .. } => {
                // No wind out in the direction of our activating button
                dir != *button_dir
            }
            Block::Output { .. } => false,
            _ => self.has_wind_hole(dir),
        }
    }

    pub fn has_move_hole(&self, dir: Dir3) -> bool {
        match self {
            Block::BlipDuplicator { out_dirs, .. } => dir != out_dirs.0 && dir != out_dirs.1,
            Block::BlipWindSource { button_dir, .. } => dir == *button_dir,
            Block::DetectorBlipDuplicator { flow_axis, .. } => dir.0 == *flow_axis,
            _ => self.has_wind_hole(dir),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct PlacedBlock {
    pub block: Block,
}

impl PlacedBlock {
    pub fn has_wind_hole(&self, dir: Dir3) -> bool {
        self.block.has_wind_hole(dir)
    }

    pub fn has_move_hole(&self, dir: Dir3) -> bool {
        self.block.has_move_hole(dir)
    }

    pub fn has_wind_hole_in(&self, dir: Dir3) -> bool {
        self.block.has_wind_hole_in(dir)
    }

    pub fn has_wind_hole_out(&self, dir: Dir3) -> bool {
        self.block.has_wind_hole_out(dir)
    }

    pub fn wind_holes_in(&self) -> Vec<Dir3> {
        Dir3::ALL
            .iter()
            .filter(|dir| self.has_wind_hole_in(**dir))
            .copied()
            .collect()
    }

    pub fn wind_holes_out(&self) -> Vec<Dir3> {
        Dir3::ALL
            .iter()
            .filter(|dir| self.has_wind_hole_out(**dir))
            .copied()
            .collect()
    }
}

pub type BlockIndex = usize;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Blocks {
    // TODO: Make private -- this should not leak for when we extend to chunks
    pub indices: Grid3<Option<BlockIndex>>,
    pub data: VecOption<(Point3, PlacedBlock)>,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Machine {
    pub blocks: Blocks,
    pub level: Option<Level>,
}

impl Machine {
    pub fn new_from_block_data(
        size: &Vector3,
        slice: &[(Point3, PlacedBlock)],
        level: &Option<Level>,
    ) -> Self {
        let mut indices = Grid3::new(*size);
        let mut data = VecOption::new();

        for (pos, placed_block) in slice {
            indices[*pos] = Some(data.add((*pos, placed_block.clone())));
        }

        let blocks = Blocks { indices, data };

        Machine {
            blocks,
            level: level.clone(),
        }
    }

    pub fn new_sandbox(size: Vector3) -> Self {
        Self {
            blocks: Blocks {
                indices: Grid3::new(size),
                data: VecOption::new(),
            },
            level: None,
        }
    }

    pub fn new_from_level(level: Level) -> Self {
        let mut machine = Self {
            blocks: Blocks {
                indices: Grid3::new(level.size),
                data: VecOption::new(),
            },
            level: Some(level.clone()),
        };

        let input_y_start = level.size.y / 2 + level.spec.input_dim() as isize / 2;

        for index in 0..level.spec.input_dim() {
            machine.set_block_at_pos(
                &Point3::new(0, input_y_start - index as isize, 0),
                Some(PlacedBlock {
                    block: Block::Input {
                        out_dir: Dir3::X_POS,
                        index,
                        inputs: Vec::new(),
                        activated: None,
                    },
                }),
            );
        }

        let output_y_start = level.size.y / 2 + level.spec.output_dim() as isize / 2;

        for index in 0..level.spec.output_dim() {
            machine.set_block_at_pos(
                &Point3::new(level.size.x - 1, output_y_start - index as isize, 0),
                Some(PlacedBlock {
                    block: Block::Output {
                        in_dir: Dir3::X_NEG,
                        index,
                        outputs: Vec::new(),
                        activated: None,
                        failed: false,
                    },
                }),
            );
        }

        machine
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
            .and_then(|id| *id)
            .map(|id| (id, &self.blocks.data[id].1))
    }

    pub fn get_block_at_pos_mut(&mut self, p: &Point3) -> Option<(BlockIndex, &mut PlacedBlock)> {
        self.blocks
            .indices
            .get(p)
            .and_then(|id| *id)
            .map(move |id| (id, &mut self.blocks.data[id].1))
    }

    pub fn block_pos_at_index(&self, index: BlockIndex) -> Point3 {
        self.blocks.data[index].0
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
}

/// Stores only the data necessary for restoring a machine.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SavedMachine {
    pub size: Vector3,
    pub block_data: Vec<(Point3, PlacedBlock)>,
    pub level: Option<Level>,
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
            level: machine.level.clone(),
        }
    }

    pub fn into_machine(self) -> Machine {
        // TODO: Make use of moving
        Machine::new_from_block_data(&self.size, &self.block_data, &self.level)
    }
}
