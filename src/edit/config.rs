use std::fmt;
use std::path::PathBuf;

use glium::glutin::VirtualKeyCode;

use crate::machine::grid::{Dir3, DirMap3};
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
    pub mirror_y_key: ModifiedKey,
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

    pub select_all_key: ModifiedKey,

    pub select_key: ModifiedKey,
    pub select_layer_bound_key: ModifiedKey,
    pub pipe_tool_key: ModifiedKey,
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
            mirror_y_key: ModifiedKey::new(VirtualKeyCode::M),
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
            select_all_key: ModifiedKey::ctrl(VirtualKeyCode::A),
            select_key: ModifiedKey::new(VirtualKeyCode::Key1),
            select_layer_bound_key: ModifiedKey::ctrl(VirtualKeyCode::Key1),
            pipe_tool_key: ModifiedKey::new(VirtualKeyCode::Key2),
            block_keys: vec![
                (
                    ModifiedKey::new(VirtualKeyCode::Key3),
                    Block::BlipSpawn {
                        out_dir: Dir3::X_POS,
                        kind: BlipKind::A,
                        num_spawns: None,
                    },
                ),
                (
                    ModifiedKey::new(VirtualKeyCode::Key4),
                    Block::BlipDuplicator {
                        out_dirs: (Dir3::X_NEG, Dir3::X_POS),
                        kind: None,
                    },
                ),
                (ModifiedKey::new(VirtualKeyCode::Key5), Block::WindSource),
                (
                    ModifiedKey::new(VirtualKeyCode::Key6),
                    Block::FunnelXY {
                        flow_dir: Dir3::X_POS,
                    },
                ),
                (
                    ModifiedKey::new(VirtualKeyCode::Key7),
                    Block::GeneralPipe(DirMap3::from_fn(|dir| {
                        dir == Dir3::Y_NEG || dir == Dir3::Y_POS
                    })),
                ),
                //(ModifiedKey::new(VirtualKeyCode::Key7), Block::Solid),
                (
                    ModifiedKey::ctrl(VirtualKeyCode::Key3),
                    Block::BlipSpawn {
                        out_dir: Dir3::X_POS,
                        kind: BlipKind::A,
                        num_spawns: Some(1),
                    },
                ),
                (
                    ModifiedKey::ctrl(VirtualKeyCode::Key4),
                    Block::BlipDuplicator {
                        out_dirs: (Dir3::X_NEG, Dir3::X_POS),
                        kind: Some(BlipKind::A),
                    },
                ),
                (
                    ModifiedKey::ctrl(VirtualKeyCode::Key5),
                    Block::BlipWindSource {
                        button_dir: Dir3::Y_NEG,
                    },
                ),
                /*(
                    ModifiedKey::ctrl(VirtualKeyCode::Key1),
                    Block::Pipe(Dir3::Y_NEG, Dir3::Y_POS),
                ),
                (
                    ModifiedKey::ctrl(VirtualKeyCode::Key2),
                    Block::Pipe(Dir3::Y_NEG, Dir3::X_POS),
                ),
                (
                    ModifiedKey::ctrl(VirtualKeyCode::Key3),
                    Block::Pipe(Dir3::Z_NEG, Dir3::Z_POS),
                ),
                (
                    ModifiedKey::ctrl(VirtualKeyCode::Key4),
                    Block::Pipe(Dir3::Z_NEG, Dir3::X_POS),
                ),
                (
                    ModifiedKey::ctrl(VirtualKeyCode::Key5),
                    Block::Pipe(Dir3::Z_POS, Dir3::X_POS),
                ),
                (ModifiedKey::ctrl(VirtualKeyCode::Key1), Block::PipeMergeXY),*/
                /*(
                    ModifiedKey::ctrl(VirtualKeyCode::Key6),
                    Block::DetectorBlipDuplicator {
                        out_dir: Dir3::X_NEG,
                        flow_axis: Axis3::Y,
                        kind: Some(BlipKind::A),
                    },
                ),
                (
                    ModifiedKey::ctrl(VirtualKeyCode::Key7),
                    Block::DetectorBlipDuplicator {
                        out_dir: Dir3::X_NEG,
                        flow_axis: Axis3::Y,
                        kind: None,
                    },
                ),*/
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
