use imgui::{im_str, ImString};

use crate::edit::editor::action::Action;
use crate::edit::Config;
use crate::edit::Mode;

const BUTTON_H: f32 = 25.0;
const BUTTON_W: f32 = 66.25;
const BG_ALPHA: f32 = 0.8;

#[derive(Clone, Debug)]
pub struct Input {
    pub config: Config,
    pub current_layer: isize,
    pub mode: Mode,
}

#[derive(Clone, Debug, Default)]
pub struct Output {
    pub actions: Vec<Action>,
}

pub fn run(input: &Input, ui: &imgui::Ui, output: &mut Output) {
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
                    ui_layers(&input, ui, output);
                });
            imgui::TreeNode::new(ui, im_str!("Modes"))
                .opened(true, imgui::Condition::FirstUseEver)
                .build(|| {
                    ui_modes(&input, ui, output);
                });
            imgui::TreeNode::new(ui, im_str!("Blocks"))
                .opened(true, imgui::Condition::FirstUseEver)
                .build(|| {
                    ui_blocks(&input, ui, output);
                });
            imgui::TreeNode::new(ui, im_str!("Actions"))
                .opened(true, imgui::Condition::FirstUseEver)
                .build(|| {
                    ui_actions(&input, ui, output);
                });
        });
}

fn ui_layers(input: &Input, ui: &imgui::Ui, output: &mut Output) {
    ui.text(&ImString::new(input.current_layer.to_string()));
    ui.same_line_with_spacing(0.0, 20.0);

    let selectable = imgui::Selectable::new(im_str!("↓")).size([20.0, 0.0]);
    if selectable.build(ui) {
        output.actions.push(Action::LayerDown);
    }
    if ui.is_item_hovered() {
        let text = format!(
            "Go down a layer.\n\nShortcut: {}",
            input.config.layer_down_key,
        );
        ui.tooltip(|| ui.text(&ImString::new(text)));
    }

    ui.same_line(0.0);
    let selectable = imgui::Selectable::new(im_str!("↑")).size([20.0, 0.0]);
    if selectable.build(ui) {
        output.actions.push(Action::LayerUp);
    }
    if ui.is_item_hovered() {
        let text = format!("Go up a layer.\n\nShortcut: {}", input.config.layer_up_key);
        ui.tooltip(|| ui.text(&ImString::new(text)));
    }
}

fn ui_modes(input: &Input, ui: &imgui::Ui, output: &mut Output) {
    ui.columns(2, im_str!("ui_modes"), false);
    ui.set_column_width(0, 50.0);

    let selection = match &input.mode {
        Mode::Select { selection, .. } => Some(selection),
        Mode::SelectClickedOnBlock { selection, .. } => Some(selection),
        Mode::RectSelect {
            existing_selection, ..
        } => Some(existing_selection),
        Mode::DragAndDrop { selection, .. } => Some(selection),
        _ => None,
    };

    ui.text_disabled(&ImString::new(format!("{}", input.config.select_key)));
    ui.next_column();
    let selectable = imgui::Selectable::new(im_str!("Select"))
        .selected(selection.as_ref().map_or(false, |s| !s.is_layer_bound()));
    if selectable.build(ui) {
        output.actions.push(Action::SelectMode);
    }
    if ui.is_item_hovered() {
        let text = format!(
            "Switch to block selection mode.\n\nShortcut: {}",
            input.config.select_key
        );
        ui.tooltip(|| ui.text(&ImString::new(text)));
    }
    ui.next_column();

    ui.text_disabled(&ImString::new(format!(
        "{}",
        input.config.select_layer_bound_key
    )));
    ui.next_column();
    let selectable = imgui::Selectable::new(im_str!("Select in layer"))
        .selected(selection.as_ref().map_or(false, |s| s.is_layer_bound()));
    if selectable.build(ui) {
        output.actions.push(Action::SelectLayerBoundMode);
    }
    if ui.is_item_hovered() {
        let text = format!(
            "Switch to selecting only in the current layer.\n\nShortcut: {}",
            input.config.select_layer_bound_key
        );
        ui.tooltip(|| ui.text(&ImString::new(text)));
    }
    ui.next_column();

    ui.text_disabled(&ImString::new(format!("{}", input.config.pipe_tool_key)));
    ui.next_column();

    let selected = match &input.mode {
        Mode::PipeTool { .. } => true,
        _ => false,
    };
    let selectable = imgui::Selectable::new(im_str!("Place pipes")).selected(selected);
    if selectable.build(ui) {
        output.actions.push(Action::PipeToolMode);
    }
    if ui.is_item_hovered() {
        let text = format!(
            "Switch to pipe placement tool.\n\nShortcut: {}",
            input.config.pipe_tool_key
        );
        ui.tooltip(|| ui.text(&ImString::new(text)));
    }

    ui.columns(1, im_str!("ui_modes_end"), false);
}

fn ui_blocks(input: &Input, ui: &imgui::Ui, output: &mut Output) {
    ui.columns(2, im_str!("ui_blocks"), false);
    ui.set_column_width(0, 50.0);

    let cur_block = match &input.mode {
        Mode::PlacePiece { piece, .. } => piece.get_singleton(),
        _ => None,
    };

    for (block_key, block) in input.config.block_keys.clone().iter() {
        ui.text_disabled(&ImString::new(format!("{}", block_key)));
        ui.next_column();

        let name = &ImString::new(block.name());
        let selected = cur_block
            .as_ref()
            .map_or(false, |(_, placed_block)| placed_block.block == *block);
        let selectable = imgui::Selectable::new(name).selected(selected);
        if selectable.build(ui) {
            output.actions.push(Action::PlaceBlockMode(block.clone()));
        }
        if ui.is_item_hovered() {
            let text = format!("{}\n\nShortcut: {}", block.description(), block_key);
            ui.tooltip(|| ui.text(&ImString::new(text)));
        }
        ui.next_column();
    }

    ui.columns(1, im_str!("ui_blocks_end"), false);
}

fn ui_actions(input: &Input, ui: &imgui::Ui, output: &mut Output) {
    if ui.button(im_str!("Undo"), [BUTTON_W, BUTTON_H]) {
        output.actions.push(Action::Undo);
    }
    if ui.is_item_hovered() {
        let text = format!("Undo the last edit.\n\nShortcut: {}", input.config.undo_key);
        ui.tooltip(|| ui.text(&ImString::new(text)));
    }

    ui.same_line(0.0);

    if ui.button(im_str!("Redo"), [BUTTON_W, BUTTON_H]) {
        output.actions.push(Action::Redo);
    }
    if ui.is_item_hovered() {
        let text = format!(
            "Take back the last undo.\n\nShortcut: {}",
            input.config.redo_key
        );
        ui.tooltip(|| ui.text(&ImString::new(text)));
    }

    if ui.button(im_str!("Copy"), [BUTTON_W, BUTTON_H]) {
        output.actions.push(Action::Copy);
    }
    if ui.is_item_hovered() {
        let text = format!(
            "Copy selected blocks.\n\nShortcut: {}",
            input.config.copy_key
        );
        ui.tooltip(|| ui.text(&ImString::new(text)));
    }

    ui.same_line(0.0);

    if ui.button(im_str!("Paste"), [BUTTON_W, BUTTON_H]) {
        output.actions.push(Action::Paste);
    }
    if ui.is_item_hovered() {
        let text = format!(
            "Start placing the last copied blocks.\n\nShortcut: {}",
            input.config.paste_key
        );
        ui.tooltip(|| ui.text(&ImString::new(text)));
    }

    if ui.button(im_str!("Cut"), [BUTTON_W, BUTTON_H]) {
        output.actions.push(Action::Cut);
    }
    if ui.is_item_hovered() {
        let text = format!(
            "Copy and remove selected blocks.\n\nShortcut: {}",
            input.config.cut_key
        );
        ui.tooltip(|| ui.text(&ImString::new(text)));
    }

    ui.same_line(0.0);

    if ui.button(im_str!("Delete"), [BUTTON_W, BUTTON_H]) {
        output.actions.push(Action::Delete);
    }
    if ui.is_item_hovered() {
        let text = format!(
            "Delete selected blocks.\n\nShortcut: {}",
            input.config.delete_key
        );
        ui.tooltip(|| ui.text(&ImString::new(text)));
    }

    if ui.button(im_str!("↻"), [BUTTON_W, BUTTON_H]) {
        output.actions.push(Action::RotateCW);
    }
    if ui.is_item_hovered() {
        let text = format!(
            "Rotate blocks to be placed clockwise.\n\nShortcut: {}",
            input.config.rotate_block_cw_key
        );
        ui.tooltip(|| ui.text(&ImString::new(text)));
    }

    ui.same_line(0.0);

    if ui.button(im_str!("↺"), [BUTTON_W, BUTTON_H]) {
        output.actions.push(Action::RotateCCW);
    }
    if ui.is_item_hovered() {
        let text = format!(
            "Rotate blocks to be placed counterclockwise.\n\nShortcut: {}",
            input.config.rotate_block_ccw_key
        );
        ui.tooltip(|| ui.text(&ImString::new(text)));
    }

    if ui.button(im_str!("Mirror Y"), [BUTTON_W, BUTTON_H]) {
        output.actions.push(Action::MirrorY)
    }
    if ui.is_item_hovered() {
        let text = format!(
            "Mirror blocks to be placed at Y axis.\n\nShortcut: {}",
            input.config.mirror_y_key
        );
        ui.tooltip(|| ui.text(&ImString::new(text)));
    }

    ui.same_line(0.0);

    if ui.button(im_str!("Color"), [BUTTON_W, BUTTON_H]) {
        output.actions.push(Action::NextKind);
    }
    if ui.is_item_hovered() {
        let text = format!(
            "Changes color of selected blocks where applicable (i.e. blip spawns and copiers).\n\nShortcut: {}",
            input.config.block_kind_key,
        );
        ui.tooltip(|| ui.text(&ImString::new(text)));
    }
}
