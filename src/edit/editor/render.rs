use nalgebra as na;

use crate::edit::{Editor, Mode, Piece};
use crate::exec::TickTime;
use crate::machine::grid;
use crate::render;
use crate::render::pipeline::RenderLists;

impl Editor {
    pub fn render(&mut self, out: &mut RenderLists) -> Result<(), glium::DrawError> {
        profile!("editor");

        let grid_size: na::Vector3<f32> = na::convert(self.machine.size());
        render::machine::render_cuboid_wireframe(
            &render::machine::Cuboid {
                center: na::Point3::from(grid_size / 2.0),
                size: grid_size,
            },
            0.1,
            &na::Vector4::new(1.0, 1.0, 1.0, 1.0),
            &mut out.solid,
        );

        render::machine::render_machine(&self.machine, &TickTime::zero(), None, out);
        render::machine::render_xy_grid(
            &self.machine.size(),
            self.current_layer as f32 + 0.01,
            &mut out.plain,
        );

        match &self.mode {
            Mode::Select(selection) => {
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
                out.ortho.add(
                    render::Object::Quad,
                    &render::pipeline::DefaultInstanceParams {
                        transform: rect_transform,
                        color: na::Vector4::new(0.3, 0.3, 0.9, 0.3),
                        ..Default::default()
                    },
                );
            }
            Mode::PlacePiece { piece, offset } => {
                if let Some(mouse_grid_pos) = self.mouse_grid_pos {
                    self.render_piece_to_place(piece, &(mouse_grid_pos + offset), out);
                }
            }
            Mode::DragAndDrop {
                selection,
                center_pos,
                rotation_xy,
                layer_offset,
            } => {
                if let Some(mouse_grid_pos) = self.mouse_grid_pos {
                    let (piece, center_pos_transformed) = self.drag_and_drop_piece_from_selection(
                        selection,
                        center_pos,
                        *rotation_xy,
                        *layer_offset,
                    );
                    let offset = mouse_grid_pos - center_pos_transformed;

                    self.render_piece_to_place(&piece, &grid::Point3::from(offset), out);

                    self.render_selection(selection, false, out);
                }
            }
            Mode::PipeTool { .. } => {
                // TODO
            }
        }

        Ok(())
    }

    fn render_selection(
        &self,
        selection: &[grid::Point3],
        highlight_last: bool,
        out: &mut RenderLists,
    ) {
        for (i, &grid_pos) in selection.iter().enumerate() {
            let color = if highlight_last && i + 1 == selection.len() {
                na::Vector4::new(0.9, 0.9, 0.0, 1.0)
            } else {
                na::Vector4::new(0.9, 0.5, 0.0, 1.0)
            };

            let grid_pos_float: na::Point3<f32> = na::convert(grid_pos);
            render::machine::render_cuboid_wireframe(
                &render::machine::Cuboid {
                    center: grid_pos_float + na::Vector3::new(0.5, 0.5, 0.51),
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
        out: &mut RenderLists,
    ) {
        let pos: na::Point3<f32> = na::convert(*pos);

        render::machine::render_cuboid_wireframe(
            &render::machine::Cuboid {
                // Slight z offset so that there is less overlap with e.g. the floor
                center: pos + na::Vector3::new(0.5, 0.5, 0.51),
                size: na::Vector3::new(1.0, 1.0, 1.0),
            },
            thickness,
            color,
            &mut out.plain,
        );
    }

    fn render_piece_to_place(
        &self,
        piece: &Piece,
        piece_pos: &grid::Point3,
        out: &mut RenderLists,
    ) {
        let mut any_pos_valid = false;

        for (pos, placed_block) in piece.iter_blocks(&piece_pos.coords) {
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

            any_pos_valid = any_pos_valid || self.machine.is_valid_pos(&pos);

            if !self.machine.is_valid_pos(&pos) || self.machine.get_block_at_pos(&pos).is_some() {
                self.render_block_wireframe(
                    &pos,
                    0.020,
                    &na::Vector4::new(0.9, 0.0, 0.0, 1.0),
                    out,
                );
            }
        }

        // Show wireframe around whole piece only if there is at
        // least one block we can place at a valid position.
        if any_pos_valid {
            let piece_pos: na::Point3<f32> = na::convert(*piece_pos);
            let wire_size: na::Vector3<f32> = na::convert(piece.grid_size());
            let wire_center = piece_pos + wire_size / 2.0;
            render::machine::render_cuboid_wireframe(
                &render::machine::Cuboid {
                    center: wire_center,
                    size: wire_size,
                },
                0.015,
                &na::Vector4::new(0.9, 0.9, 0.9, 1.0),
                &mut out.plain,
            );
        }
    }
}
