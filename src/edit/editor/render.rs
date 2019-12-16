use coarse_prof::profile;
use nalgebra as na;

use rendology::{basic_obj, BasicObj};

use crate::edit::{Editor, Mode, Piece};
use crate::exec::TickTime;
use crate::machine::{grid, Block, PlacedBlock};
use crate::render::{self, Stage};

pub const GRID_OFFSET_Z: f32 = 0.01;
pub const GRID_OFFSET_2_Z: f32 = 0.02;

impl Editor {
    pub fn render(&mut self, out: &mut Stage) -> Result<(), glium::DrawError> {
        profile!("editor");

        let grid_size: na::Vector3<f32> = na::convert(self.machine.size());
        render::machine::render_cuboid_wireframe(
            &render::machine::Cuboid {
                center: na::Point3::from(grid_size / 2.0) + na::Vector3::z() * GRID_OFFSET_2_Z,
                size: grid_size,
            },
            0.1,
            &na::Vector4::new(1.0, 1.0, 1.0, 1.0),
            &mut out.solid,
        );

        let filter = |pos| {
            // Don't render blocks that are going to be overwritten by the pipe
            // tool. Otherwise it may look a bit confusing if the same grid
            // position contains two different pipes.
            if let Mode::PipeTool { blocks, .. } = &self.mode {
                !blocks.contains_key(pos)
            } else {
                true
            }
        };

        render::machine::render_machine(&self.machine, &TickTime::zero(), None, filter, out);

        render::machine::render_xy_grid(
            &self.machine.size(),
            self.current_layer as f32 + GRID_OFFSET_Z,
            &mut out.plain,
        );

        match &self.mode {
            Mode::Select { selection, .. } => {
                self.render_selection(selection, false, out);

                if let Some(mouse_block_pos) = self.mouse_block_pos {
                    self.render_block_wireframe(
                        &mouse_block_pos,
                        0.015,
                        &na::Vector4::new(0.9, 0.9, 0.9, 1.0),
                        out,
                    );
                }
            }
            Mode::RectSelect {
                existing_selection,
                new_selection,
                start_pos,
                end_pos,
            } => {
                self.render_selection(existing_selection, false, out);
                self.render_selection(new_selection, false, out);

                let min = na::Point2::new(start_pos.x.min(end_pos.x), start_pos.y.min(end_pos.y));
                let max = na::Point2::new(start_pos.x.max(end_pos.x), start_pos.y.max(end_pos.y));

                let rect_transform =
                    na::Matrix4::new_translation(&na::Vector3::new(min.x, min.y, 0.0))
                        * na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(
                            max.x - min.x,
                            max.y - min.y,
                            1.0,
                        ));
                out.ortho[BasicObj::Quad].add(basic_obj::Instance {
                    transform: rect_transform,
                    color: na::Vector4::new(0.3, 0.3, 0.9, 0.3),
                    ..Default::default()
                });
            }
            Mode::PlacePiece { piece } => {
                if let Some(mouse_grid_pos) = self.mouse_grid_pos {
                    self.render_piece_to_place(piece, &mouse_grid_pos, out);
                }
            }
            Mode::DragAndDrop { piece, selection } => {
                if let Some(mouse_grid_pos) = self.mouse_grid_pos {
                    self.render_piece_to_place(&piece, &mouse_grid_pos, out);

                    //let selection: Vec<_> = piece.iter().map(|(pos, _)| *pos);
                    self.render_selection(&selection, false, out);
                }
            }
            Mode::PipeTool {
                last_pos,
                rotation_xy,
                blocks,
                ..
            } => {
                if let Some(mouse_grid_pos) = self.mouse_grid_pos {
                    if self.machine.is_valid_pos(&mouse_grid_pos) {
                        self.render_block_wireframe(
                            &mouse_grid_pos,
                            0.015,
                            &na::Vector4::new(0.9, 0.9, 0.9, 1.0),
                            out,
                        );

                        if last_pos.is_none()
                            && self.machine.get_block_at_pos(&mouse_grid_pos).is_none()
                        {
                            let mut block = Block::Pipe(grid::Dir3::Y_NEG, grid::Dir3::Y_POS);
                            for _ in 0..*rotation_xy {
                                block.mutate_dirs(|dir| dir.rotated_cw_xy());
                            }
                            let placed_block = PlacedBlock { block };
                            let block_center = render::machine::block_center(&mouse_grid_pos);
                            let block_transform =
                                render::machine::placed_block_transform(&placed_block);
                            render::machine::render_block(
                                &placed_block,
                                &TickTime::zero(),
                                &None,
                                &block_center,
                                &block_transform,
                                0.8,
                                out,
                            );
                        }
                    }
                }

                if let Some(last_pos) = last_pos {
                    self.render_block_wireframe(
                        &last_pos,
                        0.02,
                        &na::Vector4::new(0.2, 0.7, 0.2, 1.0),
                        out,
                    );

                    self.render_tentative_blocks(
                        blocks.iter().map(|(pos, block)| (*pos, block.clone())),
                        true,
                        out,
                    );
                }
            }
        }

        Ok(())
    }

    fn render_selection(&self, selection: &[grid::Point3], highlight_last: bool, out: &mut Stage) {
        for (i, &grid_pos) in selection.iter().enumerate() {
            let color = if highlight_last && i + 1 == selection.len() {
                na::Vector4::new(0.9, 0.9, 0.0, 1.0)
            } else {
                na::Vector4::new(0.9, 0.5, 0.0, 1.0)
            };

            let grid_pos_float: na::Point3<f32> = na::convert(grid_pos);

            render::machine::render_cuboid_wireframe(
                &render::machine::Cuboid {
                    center: grid_pos_float + na::Vector3::new(0.5, 0.5, 0.5 + GRID_OFFSET_2_Z),
                    size: na::Vector3::new(1.0, 1.0, 1.0),
                },
                0.025,
                &color,
                &mut out.plain,
            );
        }
    }

    fn render_block_wireframe(
        &self,
        pos: &grid::Point3,
        thickness: f32,
        color: &na::Vector4<f32>,
        out: &mut Stage,
    ) {
        let pos: na::Point3<f32> = na::convert(*pos);

        render::machine::render_cuboid_wireframe(
            &render::machine::Cuboid {
                // Slight z offset so that there is less overlap with e.g. the floor
                center: pos + na::Vector3::new(0.5, 0.5, 0.5 + GRID_OFFSET_2_Z),
                size: na::Vector3::new(1.0, 1.0, 1.0),
            },
            thickness,
            color,
            &mut out.plain,
        );
    }

    fn render_tentative_blocks(
        &self,
        blocks: impl Iterator<Item = (grid::Point3, PlacedBlock)>,
        wireframe_all: bool,
        out: &mut Stage,
    ) -> bool {
        let mut any_pos_valid = false;

        for (pos, placed_block) in blocks {
            let block_center = render::machine::block_center(&pos);
            let block_transform = render::machine::placed_block_transform(&placed_block);

            render::machine::render_block(
                &placed_block,
                &TickTime::zero(),
                &None,
                &block_center,
                &block_transform,
                0.8,
                out,
            );

            // TODO: Render tentative blocks as non-shadowed?

            any_pos_valid = any_pos_valid || self.machine.is_valid_pos(&pos);

            if wireframe_all {
                self.render_block_wireframe(
                    &pos,
                    0.015,
                    &na::Vector4::new(0.5, 0.5, 0.5, 1.0),
                    out,
                );
            } else if !self.machine.is_valid_pos(&pos)
                || self.machine.get_block_at_pos(&pos).is_some()
            {
                self.render_block_wireframe(
                    &pos,
                    0.020,
                    &na::Vector4::new(0.9, 0.0, 0.0, 1.0),
                    out,
                );
            }
        }

        any_pos_valid
    }

    fn render_piece_to_place(&self, piece: &Piece, piece_pos: &grid::Point3, out: &mut Stage) {
        let blocks = piece
            .iter()
            .map(|(pos, block)| (pos + piece_pos.coords, block));
        let any_pos_valid = self.render_tentative_blocks(blocks, false, out);

        // Show wireframe around whole piece only if there is at
        // least one block we can place at a valid position.
        if any_pos_valid {
            let piece_min: na::Point3<f32> = na::convert(piece.min_pos() + piece_pos.coords);
            let piece_max: na::Point3<f32> = na::convert(piece.max_pos() + piece_pos.coords);

            let wire_size = piece_max - piece_min + na::Vector3::new(1.0, 1.0, 1.0);
            let wire_center = piece_min + wire_size / 2.0;

            render::machine::render_cuboid_wireframe(
                &render::machine::Cuboid {
                    center: wire_center + na::Vector3::z() * GRID_OFFSET_2_Z,
                    size: wire_size,
                },
                0.015,
                &na::Vector4::new(0.9, 0.9, 0.9, 1.0),
                &mut out.plain,
            );
        }
    }
}
