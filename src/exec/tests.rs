use rand::Rng;

use crate::edit::piece::{Piece, Transform};
use crate::exec::{Exec, WindState};
use crate::machine::grid::{Dir3, Point3, Vector3};
use crate::machine::string_util::blocks_from_string;
use crate::machine::{grid, BlipKind, Block, Machine, PlacedBlock};

/// Test that wind flows one grid block per tick.
#[test]
fn test_straight_wind_propagation() {
    // A wind source, followed by 10 straight pipes to the right.
    let m = "
◉----------
-----------
";

    test_transform_invariant(&blocks_from_string(m), |t, exec| {
        for i in 0..=20 {
            exec.update();

            // Wind source has outgoing wind everywhere.
            for &d in &Dir3::ALL {
                assert!(wind(exec, t * (0, 0, 0)).wind_out(d));
            }

            // Pipe has outgoing wind to the right.
            for x in 1..=i.min(10) {
                assert!(wind(exec, t * (x, 0, 0)).wind_out(t * Dir3::X_POS));
            }

            // To the right, there is no wind yet.
            for x in i + 1..=10 {
                for &d in &Dir3::ALL {
                    assert!(!wind(exec, t * (x, 0, 0)).wind_out(d));
                }
            }

            // And to the bottom, there never is any wind.
            for x in 0..=10 {
                for &d in &Dir3::ALL {
                    assert!(!wind(exec, t * (x, 1, 0)).wind_out(d));
                }
            }
        }
    });
}

/// Test that funnel propagates wind in only one direction.
#[test]
fn test_funnel_wind_propagation() {
    // A wind source, followed by a funnel at x=5 and then pipes up to x=10.
    let m = "
◉----▷-----
";

    test_transform_invariant(&blocks_from_string(m), |t, exec| {
        for i in 0..20 {
            exec.update();

            // Pipes up to the funnel get outgoing wind (after some time).
            for x in 1..i.min(5) {
                assert!(wind(exec, t * (x, 0, 0)).wind_out(t * Dir3::X_POS));
            }

            // The funnel and pipes to the right never have any outgoing wind.
            for x in 5..=10 {
                for &d in &Dir3::ALL {
                    assert!(!wind(exec, t * (x, 0, 0)).wind_out(d));
                }
            }
        }
    });
}

/// Test that intersections propagate wind in all directions.
#[test]
fn test_merge_xy_wind_propagation() {
    // Intersection at (8,2), followed by 2 pipes up/right/down.
    let m = "
        | 
        |
◉-------┼--
        |
        |
";

    test_transform_invariant(&blocks_from_string(m), |t, exec| {
        for i in 0..20 {
            exec.update();

            // Flow to the right.
            for x in 1..=i.min(10) {
                assert!(wind(exec, t * (x, 2, 0)).wind_out(t * Dir3::X_POS));
            }

            // Flow up starts after 9 updates.
            assert_eq!(wind(exec, t * (8, 2, 0)).wind_out(t * Dir3::Y_NEG), i >= 8);
            assert_eq!(wind(exec, t * (8, 1, 0)).wind_out(t * Dir3::Y_NEG), i >= 9);
            assert_eq!(wind(exec, t * (8, 0, 0)).wind_out(t * Dir3::Y_NEG), i >= 10);

            // Flow down starts after 9 updates.
            assert_eq!(wind(exec, t * (8, 2, 0)).wind_out(t * Dir3::Y_POS), i >= 8);
            assert_eq!(wind(exec, t * (8, 3, 0)).wind_out(t * Dir3::Y_POS), i >= 9);
            assert_eq!(wind(exec, t * (8, 4, 0)).wind_out(t * Dir3::Y_POS), i >= 10);
        }
    });
}

fn wind(exec: &Exec, p: grid::Point3) -> &WindState {
    let (block_index, _block) = exec.machine().get_with_index(&p).unwrap();

    &exec.wind_state()[block_index]
}

fn test_transform_invariant<T>(blocks: &[(Point3, Block)], test: T)
where
    T: for<'a> Fn(&'a Transform, &'a mut Exec),
{
    let blocks = blocks
        .iter()
        .map(|(pos, block)| {
            (
                *pos,
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
