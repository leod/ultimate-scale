use imgui::{im_str, ImString};

use crate::edit::{Editor, Mode};

impl Editor {
    pub fn ui(&mut self, ui: &imgui::Ui) {
        let button_w = 140.0;
        let button_h = 25.0;
        let small_button_w = 66.25;
        let bg_alpha = 0.8;

        imgui::Window::new(im_str!("Blocks"))
            .horizontal_scrollbar(true)
            .movable(false)
            .always_auto_resize(true)
            .position([self.window_size.x - 10.0, 10.0], imgui::Condition::Always)
            .position_pivot([1.0, 0.0])
            .bg_alpha(bg_alpha)
            .build(&ui, || {
                let cur_block = match &self.mode {
                    Mode::PlacePiece { piece, .. } => piece.get_singleton(),
                    _ => None,
                };

                for (block_key, block) in self.config.block_keys.clone().iter() {
                    let name = &ImString::new(block.name());
                    let selected = cur_block
                        .as_ref()
                        .map_or(false, |(_, placed_block)| placed_block.block == *block);
                    let selectable = imgui::Selectable::new(name).selected(selected);

                    if selectable.build(ui) {
                        self.switch_to_place_block_mode(*block);
                    }

                    if ui.is_item_hovered() {
                        let text = format!("{}\n\nShortcut: {}", block.description(), block_key);
                        ui.tooltip(|| ui.text(&ImString::new(text)));
                    }
                }
            });

        imgui::Window::new(im_str!("Tools"))
            .horizontal_scrollbar(true)
            .movable(false)
            .always_auto_resize(true)
            .position([10.0, 10.0], imgui::Condition::Always)
            .bg_alpha(bg_alpha)
            .build(&ui, || {
                if ui.button(im_str!("Select"), [button_w, button_h]) {
                    self.mode = Mode::Select(Vec::new());
                }
                if ui.is_item_hovered() {
                    let text = format!(
                        "Switch to block selection mode.\n\nShortcut: {}",
                        self.config.select_key
                    );
                    ui.tooltip(|| ui.text(&ImString::new(text)));
                }
                if ui.button(im_str!("Pipe Tool"), [button_w, button_h]) {
                    self.mode = Mode::new_pipe_tool();
                }
                if ui.is_item_hovered() {
                    let text = format!("Switch to pipe placement tool.\n\nShortcut: {}", "TODO");
                    ui.tooltip(|| ui.text(&ImString::new(text)));
                }

                ui.separator();

                if ui.button(im_str!("Undo"), [small_button_w, button_h]) {
                    self.action_undo();
                }
                if ui.is_item_hovered() {
                    let text = format!("Undo the last edit.\n\nShortcut: {}", self.config.undo_key);
                    ui.tooltip(|| ui.text(&ImString::new(text)));
                }

                ui.same_line(0.0);

                if ui.button(im_str!("Redo"), [small_button_w, button_h]) {
                    self.action_redo();
                }
                if ui.is_item_hovered() {
                    let text = format!(
                        "Take back the last undo.\n\nShortcut: {}",
                        self.config.redo_key
                    );
                    ui.tooltip(|| ui.text(&ImString::new(text)));
                }

                ui.separator();

                if ui.button(im_str!("Copy"), [small_button_w, button_h]) {
                    self.action_copy();
                }
                if ui.is_item_hovered() {
                    let text = format!(
                        "Copy selected blocks.\n\nShortcut: {}",
                        self.config.copy_key
                    );
                    ui.tooltip(|| ui.text(&ImString::new(text)));
                }

                ui.same_line(0.0);

                if ui.button(im_str!("Paste"), [small_button_w, button_h]) {
                    self.action_paste();
                }
                if ui.is_item_hovered() {
                    let text = format!(
                        "Start placing the last copied blocks.\n\nShortcut: {}",
                        self.config.paste_key
                    );
                    ui.tooltip(|| ui.text(&ImString::new(text)));
                }

                if ui.button(im_str!("Cut"), [small_button_w, button_h]) {
                    self.action_cut();
                }
                if ui.is_item_hovered() {
                    let text = format!(
                        "Copy and remove selected blocks.\n\nShortcut: {}",
                        self.config.cut_key
                    );
                    ui.tooltip(|| ui.text(&ImString::new(text)));
                }

                ui.same_line(0.0);

                if ui.button(im_str!("Delete"), [small_button_w, button_h]) {
                    self.action_delete();
                }
                if ui.is_item_hovered() {
                    let text = format!(
                        "Delete selected blocks.\n\nShortcut: {}",
                        self.config.delete_key
                    );
                    ui.tooltip(|| ui.text(&ImString::new(text)));
                }

                ui.separator();

                ui.set_window_font_scale(1.5);
                if ui.button(im_str!("↻"), [small_button_w, button_h]) {
                    self.action_rotate_cw();
                }
                if ui.is_item_hovered() {
                    let text = format!(
                        "Rotate blocks to be placed clockwise.\n\nShortcut: {}",
                        self.config.rotate_block_cw_key
                    );
                    ui.tooltip(|| ui.text(&ImString::new(text)));
                }

                ui.same_line(0.0);

                if ui.button(im_str!("↺"), [small_button_w, button_h]) {
                    self.action_rotate_ccw();
                }
                if ui.is_item_hovered() {
                    let text = format!(
                        "Rotate blocks to be placed counterclockwise.\n\nShortcut: {}",
                        self.config.rotate_block_ccw_key
                    );
                    ui.tooltip(|| ui.text(&ImString::new(text)));
                }
                ui.set_window_font_scale(1.0);
            });
    }
}
