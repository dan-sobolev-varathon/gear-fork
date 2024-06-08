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

//! State-related data structures.

use core::num::NonZeroU32;

use alloc::{collections::BTreeMap, vec::Vec};
use gear_core::{
    code::InstrumentedCode,
    ids::ProgramId,
    memory::PageBuf,
    message::{ContextStore, DispatchKind, GasLimit, MessageDetails, Payload, Value},
    pages::{numerated::tree::IntervalsTree, GearPage, WasmPage},
    program::MemoryInfix,
    reservation::GasReservationMap,
};
use gprimitives::{CodeId, MessageId, H256};
use parity_scale_codec::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct HashAndLen {
    pub hash: H256,
    pub len: NonZeroU32,
}

// TODO: temporary solution to avoid lengths taking in account
impl From<H256> for HashAndLen {
    fn from(value: H256) -> Self {
        Self {
            hash: value,
            len: NonZeroU32::new(1).expect("impossible"),
        }
    }
}

#[derive(Clone, Debug, Encode, Decode)]
pub enum MaybeHash {
    Hash(HashAndLen),
    Empty,
}

// TODO: temporary solution to avoid lengths taking in account
impl From<H256> for MaybeHash {
    fn from(value: H256) -> Self {
        MaybeHash::Hash(HashAndLen::from(value))
    }
}

impl MaybeHash {
    pub fn with_hash_or_default<T: Default>(&self, f: impl FnOnce(H256) -> T) -> T {
        match &self {
            Self::Hash(HashAndLen { hash, .. }) => f(*hash),
            Self::Empty => Default::default(),
        }
    }
}

/// Hypercore program state.
#[derive(Clone, Debug, Decode, Encode)]
pub struct ProgramState {
    /// Hash of incoming message queue, see [`MessageQueue`].
    pub queue_hash: MaybeHash,
    /// Wasm memory pages allocations.
    pub allocations_hash: MaybeHash,
    /// Hash of memory pages table, see [`MemoryPages`].
    pub pages_hash: MaybeHash,
    /// Gas reservations map.
    pub gas_reservation_map_hash: MaybeHash,
    /// Program memory infix.
    pub memory_infix: MemoryInfix,
    /// Balance
    pub balance: Value,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct Dispatch {
    /// Message id.
    pub id: MessageId,
    /// Dispatch kind.
    pub kind: DispatchKind,
    /// Message source.
    pub source: ProgramId,
    /// Message payload.
    pub payload_hash: MaybeHash,
    /// Message gas limit. Required here.
    pub gas_limit: GasLimit,
    /// Message value.
    pub value: Value,
    /// Message details like reply message ID, status code, etc.
    pub details: Option<MessageDetails>,
    /// Message previous executions context.
    pub context: Option<ContextStore>,
}

#[derive(Clone, Debug, Encode, Decode, Default)]
pub struct MessageQueue(pub Vec<Dispatch>);

pub type MemoryPages = BTreeMap<GearPage, H256>;

pub type Allocations = IntervalsTree<WasmPage>;

pub trait Storage {
    fn clone_boxed(&self) -> Box<dyn Storage>;

    /// Reads program state by state hash.
    fn read_state(&self, hash: H256) -> Option<ProgramState>;

    /// Writes program state and returns its hash.
    fn write_state(&self, state: ProgramState) -> H256;

    /// Reads message queue by queue hash.
    fn read_queue(&self, hash: H256) -> Option<MessageQueue>;

    /// Writes message queue and returns its hash.
    fn write_queue(&self, queue: MessageQueue) -> H256;

    /// Reads memory pages by pages hash.
    fn read_pages(&self, hash: H256) -> Option<MemoryPages>;

    /// Writes memory pages and returns its hash.
    fn write_pages(&self, pages: MemoryPages) -> H256;

    /// Reads allocations by allocations hash.
    fn read_allocations(&self, hash: H256) -> Option<Allocations>;

    /// Writes allocations and returns its hash.
    fn write_allocations(&self, allocations: Allocations) -> H256;

    /// Reads gas reservation map by gas reservation map hash.
    fn read_gas_reservation_map(&self, hash: H256) -> Option<GasReservationMap>;

    /// Writes gas reservation map and returns its hash.
    fn write_gas_reservation_map(&self, gas_reservation_map: GasReservationMap) -> H256;

    /// Reads original code id by program id.
    fn get_program_code_id(&self, program_id: ProgramId) -> Option<CodeId>;

    /// Writes original code id by program id.
    fn set_program_code_id(&self, program_id: ProgramId, code_id: CodeId);

    /// Reads original code by code hash.
    fn read_original_code(&self, code_id: CodeId) -> Option<Vec<u8>>;

    /// Writes original code and returns its hash.
    fn write_original_code(&self, code: &[u8]) -> H256;

    /// Reads instrumented code by runtime id and original code id.
    fn read_instrumented_code(&self, runtime_id: u32, code_id: CodeId) -> Option<InstrumentedCode>;

    /// Writes instrumented code and returns its hash.
    fn write_instrumented_code(&self, runtime_id: u32, code_id: CodeId, code: InstrumentedCode);

    /// Reads payload by payload hash.
    fn read_payload(&self, hash: H256) -> Option<Payload>;

    /// Writes payload and returns its hash.
    fn write_payload(&self, payload: Payload) -> H256;

    /// Reads page data by page data hash.
    fn read_page_data(&self, hash: H256) -> Option<PageBuf>;

    /// Writes page data and returns its hash.
    fn write_page_data(&self, data: PageBuf) -> H256;
}
