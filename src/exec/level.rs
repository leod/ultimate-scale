use crate::exec::Activation;
use crate::machine::level::{self, InputsOutputs};
use crate::machine::{BlipKind, Block, BlockIndex, Machine};

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum LevelStatus {
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub struct Input {
    pub block_index: Option<BlockIndex>,
    pub num_fed: usize,
}

#[derive(Debug, Clone)]
pub struct Output {
    pub block_index: Option<BlockIndex>,
    pub num_fed: usize,
    pub failed: bool,
}

/// `LevelProgress` stores the progress through the current `InputsOutputs`
/// example while executing.
#[derive(Debug, Clone)]
pub struct LevelProgress {
    /// The `InputsOutputs` that were generated.
    pub inputs_outputs: InputsOutputs,

    /// States of each input.
    ///
    /// This vector has the same length as the level's `InputOutputs::inputs`.
    pub inputs: Vec<Input>,

    /// States of each output.
    ///
    /// This vector has the same length as the level's `InputOutputs::outputs`.
    pub outputs: Vec<Output>,
}

impl LevelProgress {
    pub fn new(machine: Option<&Machine>, inputs_outputs: InputsOutputs) -> Self {
        let inputs = inputs_outputs
            .inputs
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let block_index = machine.and_then(|machine| {
                    machine
                        .iter_blocks()
                        .find(|(_, (_, block))| {
                            if let Block::Input { index, .. } = block.block {
                                index == i
                            } else {
                                false
                            }
                        })
                        .map(|(block_index, _)| block_index)
                });

                Input {
                    block_index,
                    num_fed: 0,
                }
            })
            .collect();

        let outputs = inputs_outputs
            .outputs
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let block_index = machine.and_then(|machine| {
                    machine
                        .iter_blocks()
                        .find(|(_, (_, block))| {
                            if let Block::Output { index, .. } = block.block {
                                index == i
                            } else {
                                false
                            }
                        })
                        .map(|(block_index, _)| block_index)
                });

                Output {
                    block_index,
                    num_fed: 0,
                    failed: false,
                }
            })
            .collect();

        Self {
            inputs_outputs,
            inputs,
            outputs,
        }
    }

    pub fn feed_input(&mut self, index: usize) -> Option<BlipKind> {
        let inputs_outputs = &self.inputs_outputs;

        self.inputs.get_mut(index).and_then(|input| {
            let spec = &inputs_outputs.inputs[index];

            if input.num_fed < spec.len() {
                input.num_fed += 1;

                spec[input.num_fed - 1].map(|input| match input {
                    level::Input::Blip(kind) => kind,
                })
            } else {
                None
            }
        })
    }

    pub fn update_outputs(&mut self, next_activation: &[Activation]) {
        for (index, output) in self.outputs.iter_mut().enumerate() {
            let blip_kind = output
                .block_index
                .and_then(|block_index| next_activation[block_index]);

            if let Some(blip_kind) = blip_kind {
                let spec = &self.inputs_outputs.outputs[index];

                if output.num_fed < spec.len() && spec[output.num_fed] == blip_kind {
                    output.num_fed += 1;
                } else {
                    output.failed = true;
                }
            }
        }
    }

    pub fn expected_output(&self, index: usize) -> Option<BlipKind> {
        self.outputs.get(index).and_then(|output| {
            let spec = &self.inputs_outputs.outputs[index];

            if output.num_fed < spec.len() {
                Some(spec[output.num_fed])
            } else {
                None
            }
        })
    }

    pub fn status(&self) -> LevelStatus {
        let any_failed = self.outputs.iter().any(|output| output.failed);
        let all_finished = self
            .outputs
            .iter()
            .enumerate()
            .all(|(index, output)| output.num_fed == self.inputs_outputs.outputs[index].len());

        if any_failed {
            LevelStatus::Failed
        } else if all_finished {
            LevelStatus::Completed
        } else {
            LevelStatus::Running
        }
    }
}
