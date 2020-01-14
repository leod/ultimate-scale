use crate::exec::{Exec, Activation};
use crate::machine::{Block, BlockIndex};
use crate::machine::level::InputsOutputs;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum LevelStatus {
    Running,
    Completed,
    Failed,
}

pub struct Input {
    pub block_index: Option<BlockIndex>,
    pub num_fed: usize,
}

pub struct Output {
    pub block_index: Option<BlockIndex>,
    pub num_fed: usize,
    pub failed: bool,
}

/// `LevelProgress` stores the progress through the current `InputsOutputs`
/// example while executing.
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
    pub fn new(machine: &Machine, inputs_outputs: InputsOutputs) -> Self {
        let inputs = inputs_outputs
            .inputs
            .iter()
            .enumerate() 
            .map(|(i, _)| {
                let block_index = machine
                    .iter_blocks()
                    .find(|(_, (_, block))| {
                        if let Block::Input { index, .. } = block.block {
                            *index == i
                        } else {
                            false
                        }
                    })
                    .map(|(block_index, _)| block_index);

                Input {
                    block_index,
                    num_fed: 0,
                }
            });

        let outputs = inputs_outputs
            .outputs
            .iter()
            .enumerate() 
            .map(|(i, _)| {
                let block_index = machine
                    .iter_blocks()
                    .find(|(_, (_, block))| {
                        if let Block::Output { index, .. } = block.block {
                            *index == i
                        } else {
                            false
                        }
                    })
                    .map(|(block_index, _)| block_index);

                Output {
                    block_index,
                    num_fed: 0,
                    failed: false,
                }
            });

        Self {
            inputs_outputs,
            inputs,
            outputs,
        }
    }

    pub fn next_input(&mut self, index: usize) -> Option<BlipKind> {
        self.inputs.get_mut(index).and_then(|input| {
            let spec = &self.inputs_outputs.inputs[index];

            if input.num_fed < spec.len() {
                input.num_fed += 1;

                spec[inputs.num_fed - 1]
            } else {
                None
            }
        })
    }

    pub fn feed_output(&mut self, index: usize, blip_kind: BlipKind) {
        if let Some(output) = self.outputs.get_mut(index) {
            let spec = &self.inputs_outputs.outputs[index];

            if output.num_fed < spec.len() && spec[output.num_fed] == blip_kind {
                output.num_fed += 1;
            } else {
                output.failed = true;
            }
        }
    }

    pub fn status(&self) -> LevelStatus {
        let any_failed = self.outputs.any(|output| output.failed);
        let all_finished = self.outputs.enumerate().all(|(index, output)|
            output.num_fed == self.inputs_outputs.outputs[index].len()
        );

        if any_failed {
            LevelStatus::Failed
        } else if all_finished {
            LevelStatus::Completed
        } else {
            LevelStatus::Running
        }
    }
}
