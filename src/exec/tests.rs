use rand::Rng;

use crate::edit::piece::{Piece, Transform};
use crate::exec::{BlipSpawnMode, BlipStatus, Exec};
use crate::machine::grid::{Dir3, Point3};
use crate::machine::string_util::blocks_from_string;
use crate::machine::{grid, Block, Machine, PlacedBlock};

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
                assert!(next_wind_out(exec, t * (0, 0, 0), d));
            }

            // Pipe has outgoing wind to the right.
            for x in 1..=i.min(10) {
                println!("{}, {}", x, i);
                assert!(next_wind_out(exec, t * (x, 0, 0), t * Dir3::X_POS));
            }

            // To the right, there is no wind yet.
            for x in i + 1..=10 {
                for &d in &Dir3::ALL {
                    assert!(!next_wind_out(exec, t * (x, 0, 0), d));
                }
            }

            // And to the bottom, there never is any wind.
            for x in 0..=10 {
                for &d in &Dir3::ALL {
                    assert!(!next_wind_out(exec, t * (x, 1, 0), d));
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
                assert!(next_wind_out(exec, t * (x, 0, 0), t * Dir3::X_POS));
            }

            // The funnel and pipes to the right never have any outgoing wind.
            for x in 5..=10 {
                for &d in &Dir3::ALL {
                    assert!(!next_wind_out(exec, t * (x, 0, 0), d));
                }
            }
        }
    });
}

/// Test that intersections propagate wind in all directions.
#[test]
fn test_merge_xy_wind_propagation() {
    // Intersection at (8,2), followed by two pipes up/right/down.
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
                assert!(next_wind_out(exec, t * (x, 2, 0), t * Dir3::X_POS));
            }

            // Flow up starts after 9 updates.
            assert_eq!(next_wind_out(exec, t * (8, 2, 0), t * Dir3::Y_NEG), i >= 8);
            assert_eq!(next_wind_out(exec, t * (8, 1, 0), t * Dir3::Y_NEG), i >= 9);
            assert_eq!(next_wind_out(exec, t * (8, 0, 0), t * Dir3::Y_NEG), i >= 10);

            // Flow down starts after 9 updates.
            assert_eq!(next_wind_out(exec, t * (8, 2, 0), t * Dir3::Y_POS), i >= 8);
            assert_eq!(next_wind_out(exec, t * (8, 3, 0), t * Dir3::Y_POS), i >= 9);
            assert_eq!(next_wind_out(exec, t * (8, 4, 0), t * Dir3::Y_POS), i >= 10);
        }
    });
}

/// Test propagation of a single sliver of wind.
#[test]
fn test_wind_sliver_propagation() {
    // Singleton blip spawn at (0,2), which activates a blip wind source
    // pointing to the right. Intersection at (8,2), followed by two pipes
    // up/right/down.
    let m = "
        | 
        |
┠[------┼--
        |
        |
";

    test_transform_invariant(&blocks_from_string(m), |t, exec| {
        for i in 0..20 {
            exec.update();

            // Flow to the right.
            //
            // i=0: Blip is spawned at (1,2).
            // i=1: Wind source is activated.
            // i=2: Wind starts flowing out.
            for x in 1..=10 {
                // Note that the blip wind source is at x=1, where wind flows
                // out at i=2.
                assert_eq!(
                    next_wind_out(exec, t * (x, 2, 0), t * Dir3::X_POS),
                    i == x + 1
                );
            }

            // Flow up starts after 10 updates.
            assert_eq!(
                next_wind_out(exec, t * (8, 2, 0), t * Dir3::Y_NEG),
                i == 8 + 1
            );
            assert_eq!(
                next_wind_out(exec, t * (8, 1, 0), t * Dir3::Y_NEG),
                i == 9 + 1
            );
            assert_eq!(
                next_wind_out(exec, t * (8, 0, 0), t * Dir3::Y_NEG),
                i == 10 + 1
            );

            // Flow down starts after 10 updates.
            assert_eq!(
                next_wind_out(exec, t * (8, 2, 0), t * Dir3::Y_POS),
                i == 8 + 1
            );
            assert_eq!(
                next_wind_out(exec, t * (8, 3, 0), t * Dir3::Y_POS),
                i == 9 + 1
            );
            assert_eq!(
                next_wind_out(exec, t * (8, 4, 0), t * Dir3::Y_POS),
                i == 10 + 1
            );
        }
    });
}

/// Test blip duplicator and single blip movement.
#[test]
fn test_blip_duplicator_and_single_blip_movement() {
    // A single blip is spawned and moved into the blip duplicator at (8,1).
    let m = "
◉-------┐
 ┷     -┿-
";

    test_transform_invariant(&blocks_from_string(m), |t, exec| {
        for i in 0..20 {
            exec.update();

            // Check blip movement.
            //
            // i=0: Blip is spawned at (1,0). Wind is spawned at (0,0).
            // i=1: Blip is at (2,0).
            // etc.
            for x in 1..=8 {
                assert_eq!(blip_index(exec, t * (x, 0, 0)).is_some(), i == x - 1);
            }

            // i=8: Blip enters blip duplicator.
            // i=9: Two output blips are spawned.
            assert_eq!(blip_index(exec, t * (7, 1, 0)).is_some(), i >= 9);
            assert_eq!(blip_index(exec, t * (9, 1, 0)).is_some(), i >= 9);
        }
    });
}

/// Test blip duplicator inversion and blip movement.
#[test]
fn test_blip_duplicator_inversion_and_blip_movement() {
    // A stream of blips is spawned and moved into the blip duplicator at (8,1).
    // Then, the blip duplicator will flip the blip status at (7,1) and (9,1)
    // once per update.
    let m = "
◉-------┐
 ┻     -┿-
";

    test_transform_invariant(&blocks_from_string(m), |t, exec| {
        for i in 0..20 {
            exec.update();

            // Check blip movement.
            //
            // i=0: First blip is spawned at (1,0). Wind is spawned at (0,0).
            // i=1: First blip is at (2,0).
            // etc.
            for x in 1..=8 {
                assert_eq!(blip_index(exec, t * (x, 0, 0)).is_some(), i >= x - 1);
            }

            // i=8: First blip enters duplicator.
            // i=9: First two output blips are spawned. Next blip enters
            //      duplicator.
            // i=10: Duplicator inverts the first two output blips. Next blip
            //       enters duplicator. For visualization purposes, the first
            //       two output blips *still live*, but they are marked as
            //       dying. The actual removal happens in the next tick.
            let left_blip = blip_index(exec, t * (7, 1, 0));
            let right_blip = blip_index(exec, t * (9, 1, 0));

            assert_eq!(left_blip.is_some(), i >= 9);
            assert_eq!(right_blip.is_some(), i >= 9);

            if i >= 9 {
                let status = if (i - 9) % 2 == 0 {
                    BlipStatus::Spawning(BlipSpawnMode::Ease)
                } else {
                    BlipStatus::Dying
                };

                assert_eq!(exec.blips()[left_blip.unwrap()].status, status);
                assert_eq!(exec.blips()[right_blip.unwrap()].status, status);
            }
        }
    });
}

fn next_wind_out(exec: &Exec, p: Point3, d: Dir3) -> bool {
    let block_index = exec.machine().get_index(&p).unwrap();
    exec.next_blocks().wind_out[block_index][d]
}

fn blip_index(exec: &Exec, p: Point3) -> Option<usize> {
    exec.blips()
        .iter()
        .find(|(_, blip)| blip.pos == p)
        .map(|(blip_index, _)| blip_index)
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

        // Make sure the blocks all have non-negative positions. This is
        // currently a requirement for Machine.
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
