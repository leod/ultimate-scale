use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::machine::{grid, BlipKind};

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct Level {
    pub size: grid::Vector3,
    pub spec: Spec,
}

impl Level {}

#[derive(PartialEq, Eq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum Input {
    Blip(BlipKind),
}

#[derive(Debug, Clone)]
pub struct InputsOutputs {
    pub inputs: Vec<Vec<Option<Input>>>,
    pub outputs: Vec<Vec<BlipKind>>,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum Spec {
    Id { dim: usize },
}

pub fn gen_random_blip_kind<R: Rng + ?Sized>(rng: &mut R) -> BlipKind {
    if rng.gen() {
        BlipKind::A
    } else {
        BlipKind::B
    }
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

    pub fn description(&self) -> String {
        match self {
            Spec::Id { .. } => "Produce the same outputs as the inputs!".to_string(),
        }
    }

    pub fn generate_inputs_outputs<R: Rng + ?Sized>(&self, rng: &mut R) -> InputsOutputs {
        match self {
            Spec::Id { dim } => {
                let len: usize = rng.gen_range(5, 20);

                let outputs: Vec<Vec<_>> = (0..*dim)
                    .map(|_| (0..len).map(|_| gen_random_blip_kind(rng)).collect())
                    .collect();

                let inputs = outputs
                    .iter()
                    .map(|outputs| {
                        outputs
                            .iter()
                            .map(|kind| Some(Input::Blip(*kind)))
                            .collect()
                    })
                    .collect();

                InputsOutputs { inputs, outputs }
            }
        }
    }
}
