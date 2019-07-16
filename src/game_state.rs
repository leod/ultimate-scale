use crate::edit::Editor;
use crate::exec::ExecView;

pub enum GameState {
    Edit(Editor),
    Exec {
        exec_view: ExecView,
        editor: Editor,
    },
}

