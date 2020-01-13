use crate::exec::Exec;
use crate::machine::{level, Block};

/// `InputsOutputsProgress` stores the progress through the current
/// `InputsOutputs` example while executing. The state is entirely derived from
/// the machine's execution state. We store it, so that the user can see where
/// execution failed even while editing afterwards.
pub struct InputsOutputsProgress {
    /// How many inputs have been fed by index?
    ///
    /// This vector has the same length as the level's `InputOutputs::inputs`.
    pub inputs: Vec<usize>,

    /// How many outputs have been correctly fed by index?
    ///
    /// This vector has the same length as the level's `InputOutputs::outputs`.
    pub outputs: Vec<usize>,

    /// Which outputs have failed (in their last time step)?
    ///
    /// This vector has the same length as the level's `InputOutputs::outputs`.
    pub outputs_failed: Vec<bool>,
}

impl InputsOutputsProgress {
    pub fn new_from_exec(example: &level::InputsOutputs, exec: &Exec) -> Self {
        let machine = exec.machine();
        let inputs = example
            .inputs
            .iter()
            .enumerate()
            .map(|(i, spec)| {
                let progress = machine
                    .blocks
                    .data
                    .values()
                    .find_map(|(_block_pos, block)| {
                        // Block::Input index is assumed to be unique within
                        // the machine
                        match &block.block {
                            Block::Input { index, inputs, .. } if *index == i => {
                                // Note that `inputs` here stores the remaining
                                // inputs that will be fed into the machine.
                                Some(if spec.len() >= inputs.len() {
                                    spec.len() - inputs.len()
                                } else {
                                    // This case can only happen if `example`
                                    // comes from the wrong source, ignore
                                    0
                                })
                            }
                            _ => None,
                        }
                    });

                // Just show no progress if we ever have missing input blocks
                progress.unwrap_or(0)
            })
            .collect();

        let outputs_and_failed = example
            .outputs
            .iter()
            .enumerate()
            .map(|(i, spec)| {
                let progress = machine
                    .blocks
                    .data
                    .values()
                    .find_map(|(_block_pos, block)| {
                        // Block::Output index is assumed to be unique within
                        // the machine
                        match &block.block {
                            Block::Output {
                                index,
                                outputs,
                                failed,
                                ..
                            } if *index == i => {
                                // Note that `outputs` here stores the remaining
                                // outputs that need to come out of the machine.
                                let mut remaining = outputs.len();

                                /*// If `activated` matches the next expected
                                // output, there has been one more progress.
                                if remaining > 0
                                    && activated.is_some()
                                    && *activated == outputs.last().copied()
                                {
                                    remaining -= 1;
                                }*/

                                Some(if spec.len() >= remaining {
                                    (spec.len() - remaining, *failed)
                                } else {
                                    // This case can only happen if `example`
                                    // comes from the wrong source, ignore
                                    (0, false)
                                })
                            }
                            _ => None,
                        }
                    });

                // Just show no progress if we ever have missing input blocks
                progress.unwrap_or((0, false))
            })
            .collect::<Vec<_>>();

        Self {
            inputs,
            outputs: outputs_and_failed.iter().map(|(a, _)| *a).collect(),
            outputs_failed: outputs_and_failed.iter().map(|(_, b)| *b).collect(),
        }
    }
}
