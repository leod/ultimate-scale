use imgui::{im_str, ImString};

use crate::edit::{Editor, Mode};

const BUTTON_H: f32 = 25.0;
const BUTTON_W: f32 = 66.25;
const BG_ALPHA: f32 = 0.8;

impl Editor {
    pub fn ui(&mut self, ui: &imgui::Ui) {
        imgui::Window::new(im_str!("Editor"))
            .horizontal_scrollbar(true)
            .always_auto_resize(true)
            .position([0.0, 0.0], imgui::Condition::FirstUseEver)
            .movable(false)
            .bg_alpha(BG_ALPHA)
            .content_size([200.0, 0.0])
            .collapsible(false)
            .build(&ui, || {
                imgui::TreeNode::new(ui, im_str!("Layer"))
                    .opened(true, imgui::Condition::FirstUseEver)
                    .build(|| {
                        self.ui_layers(ui);
                    });
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

    fn ui_layers(&mut self, ui: &imgui::Ui) {
        ui.text(&ImString::new(self.current_layer.to_string()));
        ui.same_line_with_spacing(0.0, 20.0);

        let selectable = imgui::Selectable::new(im_str!("↓"))
            .disabled(!self.machine.is_valid_layer(self.current_layer - 1))
            .size([20.0, 0.0]);
        if selectable.build(ui) {
            self.action_layer_down();
        }
        if ui.is_item_hovered() {
            let text = format!(
                "Go down a layer.\n\nShortcut: {:?}",
                self.config.layer_down_key,
            );
            ui.tooltip(|| ui.text(&ImString::new(text)));
        }

        ui.same_line(0.0);
        let selectable = imgui::Selectable::new(im_str!("↑"))
            .disabled(!self.machine.is_valid_layer(self.current_layer + 1))
            .size([20.0, 0.0]);
        if selectable.build(ui) {
            self.action_layer_up();
        }
        if ui.is_item_hovered() {
            let text = format!("Go up a layer.\n\nShortcut: {:?}", self.config.layer_up_key,);
            ui.tooltip(|| ui.text(&ImString::new(text)));
        }
    }

    fn ui_modes(&mut self, ui: &imgui::Ui) {
        ui.columns(2, im_str!("ui_modes"), false);
        ui.set_column_width(0, 50.0);

        let selection = match self.mode.clone() {
            Mode::Select { selection, .. } => Some(selection),
            Mode::SelectClickedOnBlock { selection, .. } => Some(selection),
            Mode::RectSelect {
                existing_selection, ..
            } => Some(existing_selection),
            Mode::DragAndDrop { selection, .. } => Some(selection),
            _ => None,
        };

        ui.text_disabled(&ImString::new(format!("{}", self.config.select_key)));
        ui.next_column();
        let selectable = imgui::Selectable::new(im_str!("Select"))
            .selected(selection.as_ref().map_or(false, |s| !s.is_layer_bound()));
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

        ui.text_disabled(&ImString::new(format!(
            "{}",
            self.config.select_layer_bound_key
        )));
        ui.next_column();
        let selectable = imgui::Selectable::new(im_str!("Select in layer"))
            .selected(selection.as_ref().map_or(false, |s| s.is_layer_bound()));
        if selectable.build(ui) {
            self.action_select_layer_bound_mode();
        }
        if ui.is_item_hovered() {
            let text = format!(
                "Switch to selecting only in the current layer.\n\nShortcut: {}",
                self.config.select_layer_bound_key
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
                self.switch_to_place_block_mode(block.clone());
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
        if ui.button(im_str!("Undo"), [BUTTON_W, BUTTON_H]) {
            self.action_undo();
        }
        if ui.is_item_hovered() {
            let text = format!("Undo the last edit.\n\nShortcut: {}", self.config.undo_key);
            ui.tooltip(|| ui.text(&ImString::new(text)));
        }

        ui.same_line(0.0);

        if ui.button(im_str!("Redo"), [BUTTON_W, BUTTON_H]) {
            self.action_redo();
        }
        if ui.is_item_hovered() {
            let text = format!(
                "Take back the last undo.\n\nShortcut: {}",
                self.config.redo_key
            );
            ui.tooltip(|| ui.text(&ImString::new(text)));
        }

        if ui.button(im_str!("Copy"), [BUTTON_W, BUTTON_H]) {
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

        if ui.button(im_str!("Paste"), [BUTTON_W, BUTTON_H]) {
            self.action_paste();
        }
        if ui.is_item_hovered() {
            let text = format!(
                "Start placing the last copied blocks.\n\nShortcut: {}",
                self.config.paste_key
            );
            ui.tooltip(|| ui.text(&ImString::new(text)));
        }

        if ui.button(im_str!("Cut"), [BUTTON_W, BUTTON_H]) {
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

        if ui.button(im_str!("Delete"), [BUTTON_W, BUTTON_H]) {
            self.action_delete();
        }
        if ui.is_item_hovered() {
            let text = format!(
                "Delete selected blocks.\n\nShortcut: {}",
                self.config.delete_key
            );
            ui.tooltip(|| ui.text(&ImString::new(text)));
        }

        if ui.button(im_str!("↻"), [BUTTON_W, BUTTON_H]) {
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

        if ui.button(im_str!("↺"), [BUTTON_W, BUTTON_H]) {
            self.action_rotate_ccw();
        }
        if ui.is_item_hovered() {
            let text = format!(
                "Rotate blocks to be placed counterclockwise.\n\nShortcut: {}",
                self.config.rotate_block_ccw_key
            );
            ui.tooltip(|| ui.text(&ImString::new(text)));
        }

        if ui.button(im_str!("Mirror Y"), [BUTTON_W, BUTTON_H]) {
            self.action_mirror_y();
        }
        if ui.is_item_hovered() {
            let text = format!(
                "Mirror blocks to be placed at Y axis.\n\nShortcut: {}",
                self.config.mirror_y_key
            );
            ui.tooltip(|| ui.text(&ImString::new(text)));
        }

        ui.same_line(0.0);

        if ui.button(im_str!("Color"), [BUTTON_W, BUTTON_H]) {
            self.action_next_kind();
        }
        if ui.is_item_hovered() {
            let text = format!(
                "Changes color of selected blocks where applicable (i.e. blip spawns and copiers).\n\nShortcut: {}",
                self.config.block_kind_key,
            );
            ui.tooltip(|| ui.text(&ImString::new(text)));
        }
    }
}
