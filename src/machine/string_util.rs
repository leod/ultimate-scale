use crate::machine::grid::{Dir3, Point3};
use crate::machine::{BlipKind, Block};

pub fn blocks_from_string(s: &str) -> Vec<(Point3, Block)> {
    s.lines()
        .filter(|row| !row.trim().is_empty())
        .enumerate()
        .flat_map(|(y, row)| {
            row.chars().enumerate().filter_map(move |(x, c)| {
                block_from_char(c).map(|block| (Point3::new(x as isize, y as isize, 0), block))
            })
        })
        .collect()
}

pub fn block_from_char(c: char) -> Option<Block> {
    if c == '.' {
        return None;
    }

    let block = match c {
        '-' => Block::Pipe(Dir3::X_NEG, Dir3::X_POS),
        '|' => Block::Pipe(Dir3::Y_NEG, Dir3::Y_POS),
        '┘' => Block::Pipe(Dir3::X_NEG, Dir3::Y_NEG),
        '┐' => Block::Pipe(Dir3::X_NEG, Dir3::Y_POS),
        '└' => Block::Pipe(Dir3::Y_NEG, Dir3::X_POS),
        '┌' => Block::Pipe(Dir3::Y_POS, Dir3::X_POS),

        '┼' => Block::PipeMergeXY,

        '▷' => Block::FunnelXY {
            flow_dir: Dir3::X_POS,
        },
        '◁' => Block::FunnelXY {
            flow_dir: Dir3::X_NEG,
        },
        '▽' => Block::FunnelXY {
            flow_dir: Dir3::Y_POS,
        },
        '△' => Block::FunnelXY {
            flow_dir: Dir3::Y_NEG,
        },

        '◉' => Block::WindSource,

        '┻' => Block::BlipSpawn {
            out_dir: Dir3::Y_NEG,
            kind: BlipKind::A,
            num_spawns: None,
            activated: None,
        },
        '┳' => Block::BlipSpawn {
            out_dir: Dir3::Y_POS,
            kind: BlipKind::A,
            num_spawns: None,
            activated: None,
        },
        '┫' => Block::BlipSpawn {
            out_dir: Dir3::X_NEG,
            kind: BlipKind::A,
            num_spawns: None,
            activated: None,
        },
        '┣' => Block::BlipSpawn {
            out_dir: Dir3::X_POS,
            kind: BlipKind::A,
            num_spawns: None,
            activated: None,
        },

        '┷' => Block::BlipSpawn {
            out_dir: Dir3::Y_NEG,
            kind: BlipKind::A,
            num_spawns: Some(1),
            activated: None,
        },
        '┯' => Block::BlipSpawn {
            out_dir: Dir3::Y_POS,
            kind: BlipKind::A,
            num_spawns: Some(1),
            activated: None,
        },
        '┨' => Block::BlipSpawn {
            out_dir: Dir3::X_NEG,
            kind: BlipKind::A,
            num_spawns: Some(1),
            activated: None,
        },
        '┠' => Block::BlipSpawn {
            out_dir: Dir3::X_POS,
            kind: BlipKind::A,
            num_spawns: Some(1),
            activated: None,
        },

        '╂' => Block::BlipDuplicator {
            out_dirs: (Dir3::Y_NEG, Dir3::Y_POS),
            kind: None,
            activated: None,
        },
        '┿' => Block::BlipDuplicator {
            out_dirs: (Dir3::X_NEG, Dir3::X_POS),
            kind: None,
            activated: None,
        },

        '[' => Block::BlipWindSource {
            button_dir: Dir3::X_NEG,
            activated: false,
        },
        ']' => Block::BlipWindSource {
            button_dir: Dir3::X_POS,
            activated: false,
        },
        '⎵' => Block::BlipWindSource {
            button_dir: Dir3::Y_POS,
            activated: false,
        },
        '⎴' => Block::BlipWindSource {
            button_dir: Dir3::Y_NEG,
            activated: false,
        },

        '☐' => Block::Solid,

        _ => panic!("No block for {}", c),
    };

    Some(block)
}
