use crate::edit::Editor;
use crate::exec::Exec;

pub enum GameState {
    Editor(Editor),
    Exec {
        exec: Exec,
        editor: Editor,
    },
}

