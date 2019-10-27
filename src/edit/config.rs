use std::collections::HashMap;
use std::path::PathBuf;

use glium::glutin::VirtualKeyCode;

use crate::machine::grid;
use crate::machine::{BlipKind, Block};

// TODO: Shift does not work for some reason, we don't get any key press events
//       for that.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ModifiedKey {
    pub shift: bool,
    pub ctrl: bool,
    pub key: VirtualKeyCode,
}

impl ModifiedKey {
    pub fn new(key: VirtualKeyCode) -> Self {
        Self {
            shift: false,
            ctrl: false,
            key,
        }
    }

    pub fn shift(key: VirtualKeyCode) -> Self {
        Self {
            shift: true,
            ctrl: false,
            key,
        }
    }

    pub fn ctrl(key: VirtualKeyCode) -> Self {
        Self {
            shift: false,
            ctrl: true,
            key,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub default_save_path: PathBuf,
    pub rotate_block_key: ModifiedKey,
    pub block_kind_key: ModifiedKey,
    pub start_exec_key: ModifiedKey,
    pub save_key: ModifiedKey,
    pub layer_up_key: ModifiedKey,
    pub layer_down_key: ModifiedKey,
    pub block_keys: Vec<(ModifiedKey, Block)>,
    pub layer_keys: HashMap<ModifiedKey, isize>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            default_save_path: PathBuf::from("machine.json"),
            rotate_block_key: ModifiedKey::new(VirtualKeyCode::R),
            block_kind_key: ModifiedKey::new(VirtualKeyCode::C),
            start_exec_key: ModifiedKey::new(VirtualKeyCode::Space),
            save_key: ModifiedKey::ctrl(VirtualKeyCode::S),
            layer_up_key: ModifiedKey::new(VirtualKeyCode::Tab),
            layer_down_key: ModifiedKey::shift(VirtualKeyCode::Tab),
            block_keys: vec![
                (ModifiedKey::new(VirtualKeyCode::Key1), Block::PipeXY),
                (ModifiedKey::new(VirtualKeyCode::Key2), Block::PipeBendXY),
                (
                    ModifiedKey::new(VirtualKeyCode::Key3),
                    Block::PipeSplitXY {
                        open_move_hole_y: grid::Sign::Pos,
                    },
                ),
                (ModifiedKey::new(VirtualKeyCode::Key4), Block::PipeZ),
                (
                    ModifiedKey::new(VirtualKeyCode::Key5),
                    Block::PipeBendZ {
                        sign_z: grid::Sign::Pos,
                    },
                ),
                (
                    ModifiedKey::new(VirtualKeyCode::Key6),
                    Block::PipeBendZ {
                        sign_z: grid::Sign::Neg,
                    },
                ),
                (ModifiedKey::new(VirtualKeyCode::Key7), Block::FunnelXY),
                (ModifiedKey::ctrl(VirtualKeyCode::Key1), Block::WindSource),
                (
                    ModifiedKey::ctrl(VirtualKeyCode::Key2),
                    Block::BlipSpawn {
                        kind: BlipKind::A,
                        num_spawns: None,
                        activated: None,
                    },
                ),
                (
                    ModifiedKey::ctrl(VirtualKeyCode::Key3),
                    Block::BlipSpawn {
                        kind: BlipKind::A,
                        num_spawns: Some(1),
                        activated: None,
                    },
                ),
                (
                    ModifiedKey::ctrl(VirtualKeyCode::Key4),
                    Block::BlipDuplicator {
                        kind: Some(BlipKind::A),
                        activated: None,
                    },
                ),
                (
                    ModifiedKey::ctrl(VirtualKeyCode::Key5),
                    Block::BlipDuplicator {
                        kind: None,
                        activated: None,
                    },
                ),
                (
                    ModifiedKey::ctrl(VirtualKeyCode::Key6),
                    Block::BlipWindSource { activated: false },
                ),
                (ModifiedKey::ctrl(VirtualKeyCode::Key9), Block::Solid),
            ],
            layer_keys: vec![
                (ModifiedKey::new(VirtualKeyCode::F1), 0),
                (ModifiedKey::new(VirtualKeyCode::F2), 1),
                (ModifiedKey::new(VirtualKeyCode::F3), 2),
                (ModifiedKey::new(VirtualKeyCode::F4), 3),
            ]
            .into_iter()
            .collect(),
        }
    }
}
