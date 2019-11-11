use serde::{Deserialize, Serialize};

use crate::machine::{grid, BlipKind};

#[derive(PartialEq, Eq, Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Level {
    pub size: grid::Vector3,
    pub spec: Spec,
}

impl Level {}

pub struct InputOutput {
    pub input: Vec<Vec<BlipKind>>,
    pub output: Vec<Vec<BlipKind>>,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum Spec {
    Id { dim: usize },
}

impl Spec {
    pub fn input_dim(&self) -> usize {
        match *self {
            Spec::Id { dim } => dim,
        }
    }

    pub fn output_dim(&self) -> usize {
        match *self {
            Spec::Id { dim } => dim,
        }
    }

    pub fn generate_input_output(&self) -> InputOutput {
        match self {
            Spec::Id { dim } => InputOutput {
                input: Vec::new(),
                output: Vec::new(),
            },
        }
    }
}
