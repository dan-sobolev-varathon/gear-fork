// This file is part of Gear.
//
// Copyright (C) 2024 Gear Technologies Inc.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Database for hypercore.

use std::collections::BTreeMap;

use crate::{CASDatabase, KVDatabase};
use gear_core::{
    code::InstrumentedCode,
    ids::{ActorId, CodeId, ProgramId},
    memory::PageBuf,
    message::Payload,
    reservation::GasReservationMap,
};
use hypercore_runtime_common::{
    state::{Allocations, MemoryPages, MessageQueue, ProgramState, Storage, Waitlist},
    BlockInfo,
};
use parity_scale_codec::{Decode, Encode};
use primitive_types::H256;

const LOG_TARGET: &str = "hyper-db";

#[repr(u64)]
enum KeyPrefix {
    ProgramToCodeId = 0,
    InstrumentedCode = 1,
    BlockStartProgramStates = 2,
    BlockEndProgramStates = 3,
    BlockEvents = 4,
    BlockOutcome = 5,
    BlockSmallMeta = 6,
}

impl KeyPrefix {
    fn one(self, key: impl AsRef<[u8]>) -> Vec<u8> {
        [H256::from_low_u64_be(self as u64).as_bytes(), key.as_ref()].concat()
    }

    fn two(self, key1: impl AsRef<[u8]>, key2: impl AsRef<[u8]>) -> Vec<u8> {
        let key = [key1.as_ref(), key2.as_ref()].concat();
        self.one(key)
    }
}

pub struct Database {
    cas: Box<dyn CASDatabase>,
    kv: Box<dyn KVDatabase>,
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            cas: self.cas.clone_boxed(),
            kv: self.kv.clone_boxed_kv(),
        }
    }
}

#[derive(Debug, Clone, Default, Encode, Decode)]
struct BlockSmallMetaInfo {
    number_timestamp: Option<(u32, u64)>,
    parent_hash: Option<H256>,
    block_end_state_is_valid: Option<bool>,
    block_has_commitment: Option<bool>,
}

pub trait BlockMetaInfo {
    fn block_info(&self, block_hash: H256) -> Option<BlockInfo>;
    fn set_block_info(&self, block_hash: H256, block_info: BlockInfo);

    fn parent_hash(&self, block_hash: H256) -> Option<H256>;
    fn set_parent_hash(&self, block_hash: H256, parent_hash: H256);

    fn end_state_is_valid(&self, block_hash: H256) -> Option<bool>;
    fn set_end_state_is_valid(&self, block_hash: H256, is_valid: bool);

    fn block_has_commitment(&self, block_hash: H256) -> Option<bool>;
    fn set_block_has_commitment(&self, block_hash: H256, has_commitment: bool);

    fn block_start_program_states(&self, block_hash: H256) -> Option<BTreeMap<ActorId, H256>>;
    fn set_block_start_program_states(&self, block_hash: H256, map: BTreeMap<ActorId, H256>);

    fn block_end_program_states(&self, block_hash: H256) -> Option<BTreeMap<ActorId, H256>>;
    fn set_block_end_program_states(&self, block_hash: H256, map: BTreeMap<ActorId, H256>);

    fn block_events(&self, block_hash: H256) -> Option<Vec<u8>>;
    fn set_block_events(&self, block_hash: H256, events_encoded: Vec<u8>);

    fn block_outcome(&self, block_hash: H256) -> Option<Vec<u8>>;
    fn set_block_outcome(&self, block_hash: H256, outcome_encoded: Vec<u8>);
}

impl BlockMetaInfo for Database {
    fn block_info(&self, block_hash: H256) -> Option<BlockInfo> {
        self.get_block_small_meta(block_hash)
            .and_then(|meta| meta.number_timestamp)
            .map(|(number, timestamp)| BlockInfo {
                height: number,
                timestamp,
            })
    }

    fn set_block_info(&self, block_hash: H256, block_info: BlockInfo) {
        log::trace!(target: LOG_TARGET, "For block {block_hash} set: {block_info:?}");
        let BlockInfo { height, timestamp } = block_info;
        let meta = self.get_block_small_meta(block_hash).unwrap_or_default();
        self.set_block_small_meta(
            block_hash,
            BlockSmallMetaInfo {
                number_timestamp: Some((height, timestamp)),
                ..meta
            },
        );
    }

    fn parent_hash(&self, block_hash: H256) -> Option<H256> {
        self.get_block_small_meta(block_hash)
            .and_then(|meta| meta.parent_hash)
    }

    fn set_parent_hash(&self, block_hash: H256, parent_hash: H256) {
        log::trace!(target: LOG_TARGET, "For block {block_hash} set parent block: {parent_hash}");
        let meta = self.get_block_small_meta(block_hash).unwrap_or_default();
        self.set_block_small_meta(
            block_hash,
            BlockSmallMetaInfo {
                parent_hash: Some(parent_hash),
                ..meta
            },
        );
    }

    fn end_state_is_valid(&self, block_hash: H256) -> Option<bool> {
        self.get_block_small_meta(block_hash)
            .and_then(|meta| meta.block_end_state_is_valid)
    }

    fn set_end_state_is_valid(&self, block_hash: H256, is_valid: bool) {
        log::trace!(target: LOG_TARGET, "For block {block_hash} set end state valid: {is_valid}");
        let meta = self.get_block_small_meta(block_hash).unwrap_or_default();
        self.set_block_small_meta(
            block_hash,
            BlockSmallMetaInfo {
                block_end_state_is_valid: Some(is_valid),
                ..meta
            },
        );
    }

    fn block_has_commitment(&self, block_hash: H256) -> Option<bool> {
        self.get_block_small_meta(block_hash)
            .and_then(|meta| meta.block_has_commitment)
    }

    fn set_block_has_commitment(&self, block_hash: H256, has_commitment: bool) {
        log::trace!(target: LOG_TARGET, "For block {block_hash} set has commitment: {has_commitment}");
        let meta = self.get_block_small_meta(block_hash).unwrap_or_default();
        self.set_block_small_meta(
            block_hash,
            BlockSmallMetaInfo {
                block_has_commitment: Some(has_commitment),
                ..meta
            },
        );
    }

    fn block_start_program_states(&self, block_hash: H256) -> Option<BTreeMap<ActorId, H256>> {
        self.kv
            .get(&KeyPrefix::BlockStartProgramStates.one(block_hash))
            .map(|data| {
                BTreeMap::decode(&mut data.as_slice())
                    .expect("Failed to decode data into `BTreeMap`")
            })
    }

    fn set_block_start_program_states(&self, block_hash: H256, map: BTreeMap<ActorId, H256>) {
        log::trace!(target: LOG_TARGET, "For block {block_hash} set start program states: {map:?}");
        self.kv.put(
            &KeyPrefix::BlockStartProgramStates.one(block_hash),
            map.encode(),
        );
    }

    fn block_end_program_states(&self, block_hash: H256) -> Option<BTreeMap<ActorId, H256>> {
        self.kv
            .get(&KeyPrefix::BlockEndProgramStates.one(block_hash))
            .map(|data| {
                BTreeMap::decode(&mut data.as_slice())
                    .expect("Failed to decode data into `BTreeMap`")
            })
    }

    fn set_block_end_program_states(&self, block_hash: H256, map: BTreeMap<ActorId, H256>) {
        self.kv.put(
            &KeyPrefix::BlockEndProgramStates.one(block_hash),
            map.encode(),
        );
    }

    fn block_events(&self, block_hash: H256) -> Option<Vec<u8>> {
        self.kv.get(&KeyPrefix::BlockEvents.one(block_hash))
    }

    fn set_block_events(&self, block_hash: H256, events_encoded: Vec<u8>) {
        self.kv
            .put(&KeyPrefix::BlockEvents.one(block_hash), events_encoded);
    }

    fn block_outcome(&self, block_hash: H256) -> Option<Vec<u8>> {
        self.kv.get(&KeyPrefix::BlockOutcome.one(block_hash))
    }

    fn set_block_outcome(&self, block_hash: H256, outcome_encoded: Vec<u8>) {
        self.kv
            .put(&KeyPrefix::BlockOutcome.one(block_hash), outcome_encoded);
    }
}

impl Database {
    pub fn new(cas: Box<dyn CASDatabase>, kv: Box<dyn KVDatabase>) -> Self {
        Self { cas, kv }
    }

    pub fn from_one<DB: CASDatabase + KVDatabase>(db: &DB) -> Self {
        Self {
            cas: CASDatabase::clone_boxed(db),
            kv: KVDatabase::clone_boxed_kv(db),
        }
    }

    // CAS accesses.

    pub fn read_original_code(&self, code_id: CodeId) -> Option<Vec<u8>> {
        let hash = H256::from(code_id.into_bytes());
        self.cas.read(&hash)
    }

    pub fn write_original_code(&self, code: &[u8]) -> CodeId {
        self.cas.write(code).into()
    }

    // TODO: temporary solution for MVP runtime-interfaces db access.
    pub fn read_by_hash(&self, hash: H256) -> Option<Vec<u8>> {
        self.cas.read(&hash)
    }

    // TODO: temporary solution for MVP runtime-interfaces db access.
    pub fn write(&self, data: &[u8]) -> H256 {
        self.cas.write(data)
    }

    // Auxiliary code KV accesses.

    pub fn get_program_code_id(&self, program_id: ProgramId) -> Option<CodeId> {
        self.kv
            .get(&KeyPrefix::ProgramToCodeId.one(program_id))
            .map(|data| {
                CodeId::try_from(data.as_slice()).expect("Failed to decode data into `CodeId`")
            })
    }

    pub fn set_program_code_id(&self, program_id: ProgramId, code_id: CodeId) {
        self.kv.put(
            &KeyPrefix::ProgramToCodeId.one(program_id),
            code_id.into_bytes().to_vec(),
        );
    }

    pub fn read_instrumented_code(
        &self,
        runtime_id: u32,
        code_id: CodeId,
    ) -> Option<InstrumentedCode> {
        self.kv
            .get(&KeyPrefix::InstrumentedCode.two(runtime_id.to_le_bytes(), code_id))
            .map(|data| {
                InstrumentedCode::decode(&mut data.as_slice())
                    .expect("Failed to decode data into `InstrumentedCode`")
            })
    }

    pub fn write_instrumented_code(
        &self,
        runtime_id: u32,
        code_id: CodeId,
        code: InstrumentedCode,
    ) {
        self.kv.put(
            &KeyPrefix::InstrumentedCode.two(runtime_id.to_le_bytes(), code_id),
            code.encode(),
        );
    }

    fn get_block_small_meta(&self, block_hash: H256) -> Option<BlockSmallMetaInfo> {
        self.kv
            .get(&KeyPrefix::BlockSmallMeta.one(block_hash))
            .map(|data| {
                BlockSmallMetaInfo::decode(&mut data.as_slice())
                    .expect("Failed to decode data into `BlockSmallMetaInfo`")
            })
    }

    fn set_block_small_meta(&self, block_hash: H256, meta: BlockSmallMetaInfo) {
        self.kv
            .put(&KeyPrefix::BlockSmallMeta.one(block_hash), meta.encode());
    }
}

// TODO: consider to change decode panics to Results.
impl Storage for Database {
    fn read_state(&self, hash: H256) -> Option<ProgramState> {
        let data = self.cas.read(&hash)?;
        Some(
            ProgramState::decode(&mut &data[..])
                .expect("Failed to decode data into `ProgramState`"),
        )
    }

    fn write_state(&self, state: ProgramState) -> H256 {
        self.cas.write(&state.encode())
    }

    fn read_queue(&self, hash: H256) -> Option<MessageQueue> {
        let data = self.cas.read(&hash)?;
        Some(
            MessageQueue::decode(&mut &data[..])
                .expect("Failed to decode data into `MessageQueue`"),
        )
    }

    fn write_queue(&self, queue: MessageQueue) -> H256 {
        self.cas.write(&queue.encode())
    }

    fn read_waitlist(&self, hash: H256) -> Option<Waitlist> {
        self.cas.read(&hash).map(|data| {
            Waitlist::decode(&mut data.as_slice()).expect("Failed to decode data into `Waitlist`")
        })
    }

    fn write_waitlist(&self, waitlist: Waitlist) -> H256 {
        self.cas.write(&waitlist.encode())
    }

    fn read_pages(&self, hash: H256) -> Option<MemoryPages> {
        let data = self.cas.read(&hash)?;
        Some(MemoryPages::decode(&mut &data[..]).expect("Failed to decode data into `MemoryPages`"))
    }

    fn write_pages(&self, pages: MemoryPages) -> H256 {
        self.cas.write(&pages.encode())
    }

    fn read_allocations(&self, hash: H256) -> Option<Allocations> {
        let data = self.cas.read(&hash)?;
        Some(Allocations::decode(&mut &data[..]).expect("Failed to decode data into `Allocations`"))
    }

    fn write_allocations(&self, allocations: Allocations) -> H256 {
        self.cas.write(&allocations.encode())
    }

    fn read_gas_reservation_map(&self, hash: H256) -> Option<GasReservationMap> {
        let data = self.cas.read(&hash)?;
        Some(
            GasReservationMap::decode(&mut &data[..])
                .expect("Failed to decode data into `GasReservationMap`"),
        )
    }

    fn write_gas_reservation_map(&self, gas_reservation_map: GasReservationMap) -> H256 {
        self.cas.write(&gas_reservation_map.encode())
    }

    fn read_payload(&self, hash: H256) -> Option<Payload> {
        let data = self.cas.read(&hash)?;
        Some(Payload::try_from(data).expect("Failed to decode data into `Payload`"))
    }

    fn write_payload(&self, payload: Payload) -> H256 {
        self.cas.write(payload.inner())
    }

    fn read_page_data(&self, hash: H256) -> Option<PageBuf> {
        let data = self.cas.read(&hash)?;
        Some(PageBuf::decode(&mut data.as_slice()).expect("Failed to decode data into `PageBuf`"))
    }

    fn write_page_data(&self, data: PageBuf) -> H256 {
        self.cas.write(&data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database() {
        let db = crate::MemDb::default();
        let database = crate::Database::from_one(&db);

        let block_hash = H256::zero();
        let parent_hash = H256::zero();
        let map: BTreeMap<ActorId, H256> = [(ActorId::zero(), H256::zero())].into();

        database.set_block_start_program_states(block_hash, map.clone());
        assert_eq!(database.block_start_program_states(block_hash), Some(map));

        database.set_parent_hash(block_hash, parent_hash);
        assert_eq!(database.parent_hash(block_hash), Some(parent_hash));
    }
}
