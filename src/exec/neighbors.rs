use std::ops::Index;

use crate::machine::grid::DirMap3;
use crate::machine::{BlockIndex, Machine};

pub struct NeighborMap(Vec<DirMap3<Option<BlockIndex>>>);

impl NeighborMap {
    pub fn new_from_machine(machine: &Machine) -> Self {
        assert!(machine.is_contiguous());

        NeighborMap(
            machine
                .iter_blocks()
                .map(|(_, (pos, _))| {
                    DirMap3::from_fn(|dir| {
                        machine
                            .blocks
                            .indices
                            .get(&(pos + dir.to_vector()))
                            .cloned()
                            .flatten()
                    })
                })
                .collect(),
        )
    }
}

impl Index<BlockIndex> for NeighborMap {
    type Output = DirMap3<Option<BlockIndex>>;

    fn index(&self, index: BlockIndex) -> &Self::Output {
        &self.0[index]
    }
}
