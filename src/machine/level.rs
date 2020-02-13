use std::iter;

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
    Clock { pattern: Vec<BlipKind> },
    BitwiseMax,
    MakeItN { n: usize, max: usize },
    MultiplyByN { n: usize, max: usize },
}

pub fn gen_blip_kind<R: Rng + ?Sized>(rng: &mut R) -> BlipKind {
    if rng.gen() {
        BlipKind::A
    } else {
        BlipKind::B
    }
}

pub fn gen_blip_kind_seqs<R: Rng + ?Sized>(
    dim: usize,
    len: usize,
    rng: &mut R,
) -> Vec<Vec<BlipKind>> {
    (0..dim)
        .map(|_| (0..len).map(|_| gen_blip_kind(rng)).collect())
        .collect()
}

pub fn blip_input_seqs(input_kinds: &[Vec<BlipKind>]) -> Vec<Vec<Option<Input>>> {
    input_kinds
        .iter()
        .map(|row| row.iter().map(|kind| Some(Input::Blip(*kind))).collect())
        .collect()
}

impl Spec {
    pub fn input_dim(&self) -> usize {
        match *self {
            Spec::Id { dim } => dim,
            Spec::Clock { .. } => 0,
            Spec::BitwiseMax => 2,
            Spec::MakeItN { .. } => 1,
            Spec::MultiplyByN { .. } => 1,
        }
    }

    pub fn output_dim(&self) -> usize {
        match *self {
            Spec::Id { dim } => dim,
            Spec::Clock { .. } => 1,
            Spec::BitwiseMax => 1,
            Spec::MakeItN { .. } => 1,
            Spec::MultiplyByN { .. } => 1,
        }
    }

    pub fn description(&self) -> String {
        match self {
            Spec::Id { .. } => "Produce the same outputs as the inputs".to_string(),
            Spec::Clock { .. } => "Produce a repeating clock pattern".to_string(),
            Spec::BitwiseMax => format!("{} beats {}", BlipKind::B, BlipKind::A),
            Spec::MakeItN { n, .. } => format!("Round up to the next multiple of {}", n),
            Spec::MultiplyByN { n, .. } => format!("Multiply by {}", n),
        }
    }

    pub fn gen_inputs_outputs<R: Rng + ?Sized>(&self, rng: &mut R) -> InputsOutputs {
        match self {
            Spec::Id { dim } => {
                let len: usize = rng.gen_range(5, 20);
                let input_kinds = gen_blip_kind_seqs(*dim, len, rng);
                let inputs = blip_input_seqs(&input_kinds);
                let outputs = input_kinds;

                InputsOutputs { inputs, outputs }
            }
            Spec::Clock { pattern } => {
                let inputs = Vec::new();
                let outputs = vec![pattern
                    .iter()
                    .cycle()
                    .take(pattern.len() * 10)
                    .copied()
                    .collect()];

                InputsOutputs { inputs, outputs }
            }
            Spec::BitwiseMax => {
                let len: usize = rng.gen_range(5, 20);
                let input_kinds = gen_blip_kind_seqs(2, len, rng);
                let inputs = blip_input_seqs(&input_kinds);
                let outputs = vec![input_kinds[0]
                    .iter()
                    .zip(input_kinds[1].iter())
                    .map(|(a, b)| {
                        if *a == BlipKind::B || *b == BlipKind::B {
                            BlipKind::B
                        } else {
                            *a
                        }
                    })
                    .collect()];

                InputsOutputs { inputs, outputs }
            }
            Spec::MakeItN { n, max } => {
                let len_input: usize = rng.gen_range(1, *max);
                let len_output = (len_input / n + (len_input % n > 0) as usize) * n;
                let inputs = vec![iter::repeat(Some(Input::Blip(BlipKind::A)))
                    .take(len_input)
                    .collect()];
                let outputs = vec![iter::repeat(BlipKind::A).take(len_output).collect()];

                InputsOutputs { inputs, outputs }
            }
            Spec::MultiplyByN { n, max } => {
                let len_input: usize = rng.gen_range(1, *max);

                let len_output = len_input * n;
                let inputs = vec![iter::repeat(Some(Input::Blip(BlipKind::A)))
                    .take(len_input)
                    .collect()];
                let outputs = vec![iter::repeat(BlipKind::A)
                    .take(len_output)
                    .chain(iter::once(BlipKind::B))
                    .collect()];

                InputsOutputs { inputs, outputs }
            }
        }
    }
}
