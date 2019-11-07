use imgui::{im_str, ImString};

use crate::edit::{Editor, Mode};

const BUTTON_W: f32 = 140.0;
const BUTTON_H: f32 = 20.0;
const SMALL_BUTTON_W: f32 = 66.25;
const BG_ALPHA: f32 = 0.8;

impl Editor {
    pub fn ui(&mut self, ui: &imgui::Ui) {
        imgui::Window::new(im_str!("Editor"))
            .horizontal_scrollbar(true)
            .always_auto_resize(true)
            .position([10.0, 10.0], imgui::Condition::FirstUseEver)
            .bg_alpha(BG_ALPHA)
            .content_size([200.0, 0.0])
            .build(&ui, || {
                imgui::TreeNode::new(ui, im_str!("Modes"))
                    .opened(true, imgui::Condition::FirstUseEver)
                    .build(|| {
                        self.ui_modes(ui);
                    });
                imgui::TreeNode::new(ui, im_str!("Blocks"))
                    .opened(true, imgui::Condition::FirstUseEver)
                    .build(|| {
                        self.ui_blocks(ui);
                    });
                imgui::TreeNode::new(ui, im_str!("Actions"))
                    .opened(true, imgui::Condition::FirstUseEver)
                    .build(|| {
                        self.ui_actions(ui);
                    });
            });
    }

    fn ui_modes(&mut self, ui: &imgui::Ui) {
        ui.columns(2, im_str!("ui_modes"), false);
        ui.set_column_width(0, 50.0);

        ui.text_disabled(&ImString::new(format!("{}", self.config.select_key)));
        ui.next_column();

        let selected = match &self.mode {
            Mode::Select { .. } => true,
            Mode::RectSelect { .. } => true,
            Mode::DragAndDrop { .. } => true,
            _ => false,
        };
        let selectable = imgui::Selectable::new(im_str!("Select")).selected(selected);
        if selectable.build(ui) {
            self.action_select_mode();
        }
        if ui.is_item_hovered() {
            let text = format!(
                "Switch to block selection mode.\n\nShortcut: {}",
                self.config.select_key
            );
            ui.tooltip(|| ui.text(&ImString::new(text)));
        }
        ui.next_column();

        ui.text_disabled(&ImString::new(format!("{}", self.config.pipe_tool_key)));
        ui.next_column();

        let selected = match &self.mode {
            Mode::PipeTool { .. } => true,
            _ => false,
        };
        let selectable = imgui::Selectable::new(im_str!("Place pipes")).selected(selected);
        if selectable.build(ui) {
            self.action_pipe_tool_mode();
        }
        if ui.is_item_hovered() {
            let text = format!(
                "Switch to pipe placement tool.\n\nShortcut: {}",
                self.config.pipe_tool_key
            );
            ui.tooltip(|| ui.text(&ImString::new(text)));
        }

        ui.columns(1, im_str!("ui_modes_end"), false);
    }

    fn ui_blocks(&mut self, ui: &imgui::Ui) {
        ui.columns(2, im_str!("ui_blocks"), false);
        ui.set_column_width(0, 50.0);

        let cur_block = match &self.mode {
            Mode::PlacePiece { piece, .. } => piece.get_singleton(),
            _ => None,
        };

        for (block_key, block) in self.config.block_keys.clone().iter() {
            ui.text_disabled(&ImString::new(format!("{}", block_key)));
            ui.next_column();

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
            ui.next_column();
        }

        ui.columns(1, im_str!("ui_blocks_end"), false);
    }

    fn ui_actions(&mut self, ui: &imgui::Ui) {
        if ui.button(im_str!("Undo"), [SMALL_BUTTON_W, BUTTON_H]) {
            self.action_undo();
        }
        if ui.is_item_hovered() {
            let text = format!("Undo the last edit.\n\nShortcut: {}", self.config.undo_key);
            ui.tooltip(|| ui.text(&ImString::new(text)));
        }

        ui.same_line(0.0);

        if ui.button(im_str!("Redo"), [SMALL_BUTTON_W, BUTTON_H]) {
            self.action_redo();
        }
        if ui.is_item_hovered() {
            let text = format!(
                "Take back the last undo.\n\nShortcut: {}",
                self.config.redo_key
            );
            ui.tooltip(|| ui.text(&ImString::new(text)));
        }

        if ui.button(im_str!("Copy"), [SMALL_BUTTON_W, BUTTON_H]) {
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

        if ui.button(im_str!("Paste"), [SMALL_BUTTON_W, BUTTON_H]) {
            self.action_paste();
        }
        if ui.is_item_hovered() {
            let text = format!(
                "Start placing the last copied blocks.\n\nShortcut: {}",
                self.config.paste_key
            );
            ui.tooltip(|| ui.text(&ImString::new(text)));
        }

        if ui.button(im_str!("Cut"), [SMALL_BUTTON_W, BUTTON_H]) {
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

        if ui.button(im_str!("Delete"), [SMALL_BUTTON_W, BUTTON_H]) {
            self.action_delete();
        }
        if ui.is_item_hovered() {
            let text = format!(
                "Delete selected blocks.\n\nShortcut: {}",
                self.config.delete_key
            );
            ui.tooltip(|| ui.text(&ImString::new(text)));
        }

        if ui.button(im_str!("↻"), [SMALL_BUTTON_W, BUTTON_H]) {
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

        if ui.button(im_str!("↺"), [SMALL_BUTTON_W, BUTTON_H]) {
            self.action_rotate_ccw();
        }
        if ui.is_item_hovered() {
            let text = format!(
                "Rotate blocks to be placed counterclockwise.\n\nShortcut: {}",
                self.config.rotate_block_ccw_key
            );
            ui.tooltip(|| ui.text(&ImString::new(text)));
        }
    }
}
