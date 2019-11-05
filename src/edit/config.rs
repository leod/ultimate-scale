use std::fmt;
use std::path::PathBuf;

use glium::glutin::VirtualKeyCode;

use crate::machine::grid;
use crate::machine::{BlipKind, Block};

// TODO: Shift does not work for some reason, we don't get any key press events
//       for that.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ModifiedKey {
    pub ctrl: bool,
    pub shift: bool,
    pub key: VirtualKeyCode,
}

impl ModifiedKey {
    pub fn new(key: VirtualKeyCode) -> Self {
        Self {
            ctrl: false,
            shift: false,
            key,
        }
    }

    pub fn shift(key: VirtualKeyCode) -> Self {
        Self {
            ctrl: false,
            shift: true,
            key,
        }
    }

    pub fn ctrl(key: VirtualKeyCode) -> Self {
        Self {
            ctrl: true,
            shift: false,
            key,
        }
    }
}

impl fmt::Display for ModifiedKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.ctrl {
            write!(f, "Ctrl-")?;
        }
        if self.shift {
            write!(f, "Shift-")?;
        }

        let key = match self.key {
            VirtualKeyCode::Key1 => "1".to_string(),
            VirtualKeyCode::Key2 => "2".to_string(),
            VirtualKeyCode::Key3 => "3".to_string(),
            VirtualKeyCode::Key4 => "4".to_string(),
            VirtualKeyCode::Key5 => "5".to_string(),
            VirtualKeyCode::Key6 => "6".to_string(),
            VirtualKeyCode::Key7 => "7".to_string(),
            VirtualKeyCode::Key8 => "8".to_string(),
            VirtualKeyCode::Key9 => "9".to_string(),
            VirtualKeyCode::Key0 => "0".to_string(),
            key => format!("{:?}", key),
        };

        write!(f, "{}", key)
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub default_save_path: PathBuf,

    pub cancel_key: ModifiedKey,

    pub rotate_block_cw_key: ModifiedKey,
    pub rotate_block_ccw_key: ModifiedKey,
    pub block_kind_key: ModifiedKey,

    pub undo_key: ModifiedKey,
    pub redo_key: ModifiedKey,

    pub copy_key: ModifiedKey,
    pub paste_key: ModifiedKey,
    pub cut_key: ModifiedKey,
    pub delete_key: ModifiedKey,

    pub save_key: ModifiedKey,

    pub layer_up_key: ModifiedKey,
    pub layer_down_key: ModifiedKey,
    pub select_key: ModifiedKey,
    pub block_keys: Vec<(ModifiedKey, Block)>,
    pub layer_keys: Vec<(ModifiedKey, isize)>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            default_save_path: PathBuf::from("machine.json"),
            cancel_key: ModifiedKey::new(VirtualKeyCode::Escape),
            rotate_block_cw_key: ModifiedKey::new(VirtualKeyCode::R),
            rotate_block_ccw_key: ModifiedKey::shift(VirtualKeyCode::R),
            block_kind_key: ModifiedKey::new(VirtualKeyCode::C),
            undo_key: ModifiedKey::ctrl(VirtualKeyCode::Z),
            redo_key: ModifiedKey::ctrl(VirtualKeyCode::Y),
            copy_key: ModifiedKey::ctrl(VirtualKeyCode::C),
            paste_key: ModifiedKey::ctrl(VirtualKeyCode::V),
            cut_key: ModifiedKey::ctrl(VirtualKeyCode::X),
            delete_key: ModifiedKey::new(VirtualKeyCode::Delete),
            save_key: ModifiedKey::ctrl(VirtualKeyCode::S),
            layer_up_key: ModifiedKey::new(VirtualKeyCode::Tab),
            layer_down_key: ModifiedKey::shift(VirtualKeyCode::Tab),
            select_key: ModifiedKey::new(VirtualKeyCode::Key1),
            block_keys: vec![
                (ModifiedKey::new(VirtualKeyCode::Key2), Block::PipeXY),
                (ModifiedKey::new(VirtualKeyCode::Key3), Block::PipeBendXY),
                (
                    ModifiedKey::new(VirtualKeyCode::Key4),
                    Block::PipeSplitXY {
                        open_move_hole_y: grid::Sign::Pos,
                    },
                ),
                (ModifiedKey::new(VirtualKeyCode::Key5), Block::PipeZ),
                (
                    ModifiedKey::new(VirtualKeyCode::Key6),
                    Block::PipeBendZ {
                        sign_z: grid::Sign::Pos,
                    },
                ),
                (
                    ModifiedKey::new(VirtualKeyCode::Key7),
                    Block::PipeBendZ {
                        sign_z: grid::Sign::Neg,
                    },
                ),
                (ModifiedKey::new(VirtualKeyCode::Key8), Block::PipeMergeXY),
                (ModifiedKey::new(VirtualKeyCode::Key9), Block::FunnelXY),
                (ModifiedKey::ctrl(VirtualKeyCode::Key2), Block::WindSource),
                (
                    ModifiedKey::ctrl(VirtualKeyCode::Key3),
                    Block::BlipSpawn {
                        kind: BlipKind::A,
                        num_spawns: None,
                        activated: None,
                    },
                ),
                (
                    ModifiedKey::ctrl(VirtualKeyCode::Key4),
                    Block::BlipSpawn {
                        kind: BlipKind::A,
                        num_spawns: Some(1),
                        activated: None,
                    },
                ),
                (
                    ModifiedKey::ctrl(VirtualKeyCode::Key5),
                    Block::BlipDuplicator {
                        kind: Some(BlipKind::A),
                        activated: None,
                    },
                ),
                (
                    ModifiedKey::ctrl(VirtualKeyCode::Key6),
                    Block::BlipDuplicator {
                        kind: None,
                        activated: None,
                    },
                ),
                (
                    ModifiedKey::ctrl(VirtualKeyCode::Key7),
                    Block::BlipWindSource { activated: false },
                ),
                (ModifiedKey::ctrl(VirtualKeyCode::Key9), Block::Solid),
            ],
            layer_keys: vec![
                (ModifiedKey::new(VirtualKeyCode::F1), 0),
                (ModifiedKey::new(VirtualKeyCode::F2), 1),
                (ModifiedKey::new(VirtualKeyCode::F3), 2),
                (ModifiedKey::new(VirtualKeyCode::F4), 3),
            ],
        }
    }
}
