use rand::Rng;

use crate::edit::piece::{Piece, Transform};
use crate::exec::Exec;
use crate::machine::grid::{Dir3, Point3, Vector3};
use crate::machine::{grid, Block, Machine, PlacedBlock};

use Block::*;

type Blocks = [((isize, isize, isize), Block)];

fn machine(blocks: &Blocks) -> Machine {
    let blocks: Vec<(Point3, PlacedBlock)> = blocks
        .iter()
        .map(|(pos, block)| {
            (
                Point3::new(pos.0, pos.1, pos.2),
                PlacedBlock {
                    block: block.clone(),
                },
            )
        })
        .collect();
    let size_x = blocks.iter().map(|(pos, _)| pos.x).max().unwrap() + 1;
    let size_y = blocks.iter().map(|(pos, _)| pos.y).max().unwrap() + 1;
    let size_z = blocks.iter().map(|(pos, _)| pos.z).max().unwrap() + 1;

    Machine::new_from_block_data(&Vector3::new(size_x, size_y, size_z), &blocks, &None)
}

#[test]
fn simple_wind() {
    let b = machine(&[
        ((0, 0, 0), Pipe(Dir3::X_NEG, Dir3::X_POS)),
        ((0, 1, 0), Pipe(Dir3::X_NEG, Dir3::X_POS)),
        ((0, 2, 0), Pipe(Dir3::X_NEG, Dir3::X_POS)),
        ((0, 3, 0), Pipe(Dir3::X_NEG, Dir3::X_POS)),
    ]);

    assert!(true);
}

fn test_transform_invariance<T>(blocks: &Blocks, test: T)
where
    T: for<'a> Fn(&'a Transform, &'a mut Exec),
{
    let blocks = blocks
        .iter()
        .map(|(pos, block)| {
            (
                Point3::new(pos.0, pos.1, pos.2),
                PlacedBlock {
                    block: block.clone(),
                },
            )
        })
        .collect();
    let piece = Piece::new(blocks);

    for _ in 0..1000 {
        let mut transform = random_transform();
        let mut transformed_piece = piece.clone();
        transformed_piece.transform(&transform);

        // Make sure the blocks all have non-negative positions. This is currently a
        // requirement for Machine.
        let shift = random_shift_to_non_negative(&transformed_piece.min_pos());
        transformed_piece.transform(&shift);
        transform = Transform::Seq(vec![transform, shift]);

        let size = transformed_piece.max_pos() + grid::Vector3::new(1, 1, 1);
        let machine = Machine::new_from_block_data(&size.coords, transformed_piece.blocks(), &None);

        let mut rng = rand::thread_rng();
        let mut exec = Exec::new(machine, &mut rng);
        test(&transform, &mut exec);
    }
}

fn random_transform() -> Transform {
    const MAX_TRANSFORMS: usize = 5;
    const MAX_SHIFT_XY: isize = 100;
    const MAX_SHIFT_Z: isize = 3;

    let mut rng = rand::thread_rng();

    let transforms = (0..rng.gen_range(0, MAX_TRANSFORMS))
        .map(|_| match rng.gen_range(0, 4) {
            0 => {
                let x = rng.gen_range(-MAX_SHIFT_XY, MAX_SHIFT_XY);
                let y = rng.gen_range(-MAX_SHIFT_XY, MAX_SHIFT_XY);
                let z = rng.gen_range(-MAX_SHIFT_Z, MAX_SHIFT_Z);

                Transform::Shift(grid::Vector3::new(x, y, z))
            }
            1 => Transform::RotateCWXY,
            2 => Transform::RotateCCWXY,
            3 => Transform::MirrorY,
            _ => unreachable!(),
        })
        .collect();

    Transform::Seq(transforms)
}

fn random_shift_to_non_negative(min_pos: &grid::Point3) -> Transform {
    const MAX_SHIFT_XY: isize = 100;
    const MAX_SHIFT_Z: isize = 3;

    let mut rng = rand::thread_rng();

    let shift_x = if min_pos.x < 0 {
        -min_pos.x + rng.gen_range(0, MAX_SHIFT_XY)
    } else {
        0
    };
    let shift_y = if min_pos.y < 0 {
        -min_pos.y + rng.gen_range(0, MAX_SHIFT_XY)
    } else {
        0
    };
    let shift_z = if min_pos.z < 0 {
        -min_pos.z + rng.gen_range(0, MAX_SHIFT_Z)
    } else {
        0
    };

    Transform::Shift(grid::Vector3::new(shift_x, shift_y, shift_z))
}
