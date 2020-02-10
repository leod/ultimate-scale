use coarse_prof::profile;
use nalgebra as na;

use rendology::{basic_obj, BasicObj};

use crate::edit::{Editor, Mode, Piece};
use crate::exec::TickTime;
use crate::machine::{grid, Block, PlacedBlock};
use crate::render::{self, Stage};

pub const GRID_OFFSET_Z: f32 = 0.00;

impl Editor {
    pub fn render(&mut self, out: &mut Stage) {
        profile!("editor");

        let grid_size: na::Vector3<f32> = na::convert(self.machine.size());
        render::machine::render_cuboid_wireframe(
            &render::machine::Cuboid {
                center: na::Point3::from(grid_size / 2.0) + na::Vector3::z() * GRID_OFFSET_Z,
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

        let unfocus = |pos: &grid::Point3| {
            if self.mode.is_layer_bound() {
                pos.z != self.current_layer
            } else {
                false
            }
        };

        render::machine::render_machine(
            &self.machine,
            &TickTime::zero(),
            None,
            filter,
            unfocus,
            out,
        );

        /*render::machine::render_xy_grid(
            &self.machine.size(),
            self.current_layer as f32 + GRID_OFFSET_Z,
            &mut out.lines,
        );*/

        match &self.mode {
            Mode::Select { selection, .. } => {
                self.render_selection(selection.iter(), out);

                if let Some(mouse_block_pos) = self.mouse_block_pos {
                    self.render_block_wireframe(
                        &mouse_block_pos,
                        1.0,
                        9.0,
                        &na::Vector4::new(0.9, 0.9, 0.9, 1.0),
                        out,
                    );

                    self.render_base(&mouse_block_pos, na::Vector2::new(1, 1), out);
                }
            }
            Mode::SelectClickedOnBlock {
                selection,
                dragged_block_pos,
                ..
            } => {
                self.render_selection(selection.iter(), out);

                self.render_base(dragged_block_pos, na::Vector2::new(1, 1), out);
            }
            Mode::RectSelect {
                existing_selection,
                new_selection,
                start_pos,
                end_pos,
            } => {
                self.render_selection(existing_selection.iter(), out);
                self.render_selection(new_selection.iter(), out);

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
            Mode::PlacePiece { piece, .. } => {
                if let Some(mouse_grid_pos) = self.mouse_grid_pos {
                    self.render_piece_to_place(piece, &mouse_grid_pos, out);
                }
            }
            Mode::DragAndDrop { piece, selection } => {
                if let Some(mouse_grid_pos) = self.mouse_grid_pos {
                    self.render_piece_to_place(&piece, &mouse_grid_pos, out);

                    //let selection: Vec<_> = piece.iter().map(|(pos, _)| *pos);
                    self.render_selection(selection.iter(), out);
                }
            }
            Mode::PipeTool {
                last_pos, blocks, ..
            } => {
                let mouse_grid_pos = self.mouse_grid_pos.filter(|p| self.machine.is_valid_pos(p));

                if let Some(mouse_grid_pos) = mouse_grid_pos {
                    // Preview the available connections at the mouse position.
                    let show_outgoing = self.machine.is_block_at(&mouse_grid_pos) || last_pos.is_some();
                    let available_dirs = grid::DirMap3::from_fn(|dir| {
                        let neighbor_pos = mouse_grid_pos + dir.to_vector();

                        let is_valid_neighbor = self.machine.is_valid_pos(&neighbor_pos);
                        let can_connect_mouse = self
                            .machine
                            .get(&mouse_grid_pos)
                            .map_or(true, |block| block.block.can_connect_by_pipe(dir));
                        let can_connect_neighbor = self
                            .machine
                            .get(&neighbor_pos)
                            .map_or(true, |block| block.block.can_connect_by_pipe(dir.invert()));

                         show_outgoing && is_valid_neighbor && can_connect_mouse && can_connect_neighbor
                    });

                    if available_dirs.values().any(|b| *b) || !self.machine.is_block_at(&mouse_grid_pos) {
                        let placed_block = PlacedBlock {
                            block: Block::GeneralPipe(available_dirs),
                        };
                        let block_center = render::machine::block_center(&mouse_grid_pos);
                        let block_transform =
                            render::machine::placed_block_transform(&placed_block);
                        out.dither = true;
                        render::machine::render_block(
                            &placed_block,
                            &TickTime::zero(),
                            None,
                            None,
                            None,
                            &block_center,
                            &block_transform,
                            0.5,
                            out,
                        );
                        out.dither = false;
                    }

                    self.render_base(&mouse_grid_pos, na::Vector2::new(1, 1), out);
                }

                if let Some(last_pos) = last_pos {
                    // Pipe tool is running.
                    self.render_block_wireframe(
                        &last_pos,
                        1.0,
                        20.0,
                        &na::Vector4::new(0.2, 0.7, 0.2, 1.0),
                        out,
                    );

                    self.render_tentative_blocks(
                        blocks.iter().map(|(pos, block)| (*pos, block.clone())),
                        false,
                        out,
                    );

                    for (pos, _) in blocks.iter() {
                        if *pos != *last_pos {
                            self.render_block_wireframe(
                                &pos,
                                0.7,
                                7.0,
                                &na::Vector4::new(0.6, 0.6, 0.6, 1.0),
                                out,
                            );
                        }
                    }
                } else if let Some(mouse_grid_pos) = mouse_grid_pos {
                    // Pipe tool has not started yet.
                    self.render_block_wireframe(
                        &mouse_grid_pos,
                        1.0,
                        20.0,
                        &na::Vector4::new(0.2, 0.6, 0.2, 1.0),
                        out,
                    );
                }
            }
        }
    }

    fn render_selection<'a>(
        &self,
        selection: impl Iterator<Item = &'a grid::Point3>,
        out: &mut Stage,
    ) {
        for grid_pos in selection {
            let color = na::Vector4::new(0.9, 0.5, 0.0, 1.0);

            self.render_block_wireframe(grid_pos, 0.7, 15.0, &color, out);
        }
    }

    fn render_tentative_blocks(
        &self,
        blocks: impl Iterator<Item = (grid::Point3, PlacedBlock)>,
        show_invalid: bool,
        out: &mut Stage,
    ) -> bool {
        let mut any_pos_valid = false;

        for (pos, placed_block) in blocks {
            let block_center = render::machine::block_center(&pos);
            let block_transform = render::machine::placed_block_transform(&placed_block);

            render::machine::render_block(
                &placed_block,
                &TickTime::zero(),
                None,
                None,
                None,
                &block_center,
                &block_transform,
                0.8,
                out,
            );

            // TODO: Render tentative blocks as non-shadowed?

            let is_valid = self.machine.is_valid_pos(&pos);
            let can_place = !self.machine.is_block_at(&pos);
            let can_combine = self.machine.get(&pos).map_or(false, |old_placed_block| {
                old_placed_block
                    .block
                    .combine(&placed_block.block)
                    .is_some()
            });

            if show_invalid {
                if !is_valid || (!can_place && !can_combine) {
                    self.render_block_wireframe(
                        &pos,
                        0.9,
                        20.0,
                        &na::Vector4::new(0.9, 0.0, 0.0, 1.0),
                        out,
                    );
                } else if can_combine {
                    self.render_block_wireframe(
                        &pos,
                        1.0,
                        20.0,
                        &na::Vector4::new(1.0, 1.0, 1.0, 1.0),
                        out,
                    );
                }
            }

            any_pos_valid = any_pos_valid || is_valid;
        }

        any_pos_valid
    }

    fn render_block_wireframe(
        &self,
        pos: &grid::Point3,
        size: f32,
        thickness: f32,
        color: &na::Vector4<f32>,
        out: &mut Stage,
    ) {
        let pos: na::Point3<f32> = na::convert(*pos);
        let center = pos + na::Vector3::new(0.5, 0.5, 0.5 + GRID_OFFSET_Z);
        let transform =
            na::Matrix4::new_translation(&center.coords) * na::Matrix4::new_scaling(size);

        render::machine::render_line_wireframe(thickness, color, &transform, out);
    }

    fn render_base(&self, min_pos: &grid::Point3, size: na::Vector2<isize>, out: &mut Stage) {
        for z in 0..min_pos.z {
            let start = na::Point3::new(min_pos.x as f32, min_pos.y as f32, z as f32);
            let size = na::Vector3::new(size.x as f32, size.y as f32, 1.0);
            let center = start + size / 2.0 + na::Vector3::z() * GRID_OFFSET_Z;
            let transform = na::Matrix4::new_translation(&center.coords)
                * na::Matrix4::new_nonuniform_scaling(&size);

            render::machine::render_line_wireframe(
                5.0,
                &na::Vector4::new(0.915, 0.554, 0.547, 1.0),
                &transform,
                out,
            );
        }
    }

    fn render_piece_base(&self, piece: &Piece, piece_pos: &grid::Point3, out: &mut Stage) {
        self.render_base(
            &(piece.min_pos() + piece_pos.coords),
            na::Vector2::new(piece.extent().x, piece.extent().y),
            out,
        );
    }

    fn render_piece_to_place(&self, piece: &Piece, piece_pos: &grid::Point3, out: &mut Stage) {
        let blocks = piece
            .iter()
            .map(|(pos, block)| (pos + piece_pos.coords, block));
        let any_pos_valid = self.render_tentative_blocks(blocks, true, out);

        // Show how far above zero the piece is.
        self.render_piece_base(piece, piece_pos, out);

        // Show wireframe around whole piece only if there is at
        // least one block we can place at a valid position.
        if any_pos_valid {
            let piece_min: na::Point3<f32> = na::convert(piece.min_pos() + piece_pos.coords);
            let piece_max: na::Point3<f32> = na::convert(piece.max_pos() + piece_pos.coords);

            let wire_size = piece_max - piece_min + na::Vector3::new(1.0, 1.0, 1.0);
            let wire_center = piece_min + wire_size / 2.0;
            let transform = na::Matrix4::new_translation(&wire_center.coords)
                * na::Matrix4::new_nonuniform_scaling(&wire_size);

            render::machine::render_line_wireframe(
                10.0,
                &na::Vector4::new(0.9, 0.9, 0.9, 1.0),
                &transform,
                out,
            );
        }
    }
}
