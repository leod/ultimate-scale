use std::ops::Index;

use crate::machine::grid::{Dir3, DirMap3};

pub struct NeighborMap(Vec<DirMap3<Option<BlockIndex>>>);

impl NeighborMap {
    pub fn new_from_machine(machine: &Machine) -> Self {
        assert!(machine.is_contiguous());

        NeighborMap(
            machine
                .iter_blocks()
                .map(|(_, (pos, _))| {
                    DirMap3::from_fn(|dir|
                        machine
                            .blocks
                            .indices
                            .get(pos + dir.to_vector())
                            .flatten()
                            .cloned()
                    )
                })
        )
    }

    pub fn lookup(&self, block_index: BlockIndex, dir: Dir3) -> Option<BlockIndex> {
        self.0[block_index][dir.to_index()]
    }

    pub fn iter(&self, block_index: BlockIndex) -> impl Iterator<Item=(Dir3, BlockIndex)> {
        let neighbors = &self.0[block_index];

        self.0[block_index]
            .iter()
            .filter_map(|(dir, neighbor_index)| neighbor_index.map(|i| (dir, *i))
    }
}
