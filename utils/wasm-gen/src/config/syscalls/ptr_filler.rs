// This file is part of Gear.

// Copyright (C) 2021-2023 Gear Technologies Inc.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Entities describing config of pointer writes that `wasm-gen`
//! includes into generated wasms. They're used to set values of some
//! syscall parameters that are passed as pointer to the data.

use arbitrary::{Result, Unstructured};
use gear_wasm_instrument::syscalls::HashType;
use gsys::Hash;
use std::{collections::HashMap, mem, ops::RangeInclusive};

pub use gear_wasm_instrument::syscalls::Ptr;

/// Amount of words required to write the `Hash` to the memory.
///
/// So if `Hash` is 32 bytes on a 32 bit (4 bytes) memory word size system,
/// 8 words will be used to store the `Hash` value in the memory.
const HASH_WORDS_COUNT: usize = mem::size_of::<Hash>() / mem::size_of::<i32>();

/// Pointer filler config.
///
/// Determines what data should be written into the pointers of
/// particular type. For example, it can be configured to write
/// some data (generated in [`PointerWriteData::generate_data_to_write`])
/// into all the pointers of type `*const/*mut Value` among syscall
/// parameters.
///
/// # Note
///
/// This config will not work for [`PtrType::BufferStart`].
#[derive(Debug, Clone)]
pub struct PtrParamFillersConfig(HashMap<Ptr, PtrParamFiller>);

impl Default for PtrParamFillersConfig {
    fn default() -> PtrParamFillersConfig {
        let mut this = Self::empty();
        this.set_rule(PtrParamDataGenerator::Value(0..=100_000_000_000));
        for ty in HashType::all() {
            this.set_rule(PtrParamDataGenerator::HashWithValue {
                ty,
                value: 0..=100_000_000_000,
            });
        }
        this.set_rule(PtrParamDataGenerator::TwoHashesWithValue {
            ty1: HashType::ReservationId,
            ty2: HashType::ActorId,
            value: 0..=100_000_000_000,
        });

        this
    }
}

impl PtrParamFillersConfig {
    pub fn empty() -> PtrParamFillersConfig {
        PtrParamFillersConfig(HashMap::new())
    }

    /// Set the `PointerWrite`s for the specified pointer type.
    pub fn set_rule(&mut self, ptr_data: PtrParamDataGenerator) {
        use Ptr::*;

        let ptr = ptr_data.clone().into();
        let filler = match ptr {
            Value => {
                PtrParamFiller {
                    value_offset: 0,
                    ptr_data,
                }
            },
            HashWithValue(_) => {
                PtrParamFiller {
                    value_offset: HASH_WORDS_COUNT,
                    ptr_data,
                }
            },
            TwoHashesWithValue(_, _) => {
                PtrParamFiller {
                    value_offset: 2 * HASH_WORDS_COUNT,
                    ptr_data,
                }
            },
            Hash(_) | TwoHashes(_, _) => todo!("Currently unsupported defining ptr param filler config for `Hash` and `TwoHashes`."),
            BlockNumber
            | BlockTimestamp
            | SizedBufferStart { .. }
            | BufferStart
            | Gas
            | Length
            | BlockNumberWithHash(_)
            => panic!("Impossible to set rules for param defined by `SyscallsParamsConfig`."),
            MutBlockNumber
            | MutBlockTimestamp
            | MutSizedBufferStart { .. }
            | MutBufferStart
            | MutHash(_)
            | MutGas
            | MutLength
            | MutValue
            | MutBlockNumberWithHash(_)
            | MutHashWithValue(_)
            | MutTwoHashes(_, _)
            | MutTwoHashesWithValue(_, _) => panic!("Mutable pointers values are set by executor, not by wasm itself."),
        };

        self.0.insert(ptr, filler);
    }

    pub fn get_rule(&self, ptr: Ptr) -> Option<PtrParamFiller> {
        self.0.get(&ptr).cloned()
    }
}

/// Generates single chunk of data being written into the pointer address
/// with specified offset.
///
/// # Note:
///
/// Offset is relative to the actual pointer address that's set
/// in syscall invoke instructions.
#[derive(Debug, Clone)]
pub(crate) struct PtrParamFiller {
    /// TODO describe
    pub(crate) value_offset: usize,
    pub(crate) ptr_data: PtrParamDataGenerator,
}

/// Config for values being written into the pointer address. The
/// actual data can be generated by calling
/// [`PointerWriteData::generate_data_to_write`].
#[derive(Debug, Clone)]
pub enum PtrParamDataGenerator {
    Value(RangeInclusive<u128>),
    HashWithValue {
        ty: HashType,
        value: RangeInclusive<u128>,
        // TODO: add todo for hash data.
        // hash: [u8; 32]
    },
    TwoHashesWithValue {
        ty1: HashType,
        ty2: HashType,
        value: RangeInclusive<u128>,
        // hash1: [u8; 32]
        // hash2: [u8; 32]
    },
}

impl PtrParamDataGenerator {
    /// Get the actual data that should be written into the memory.
    pub fn generate(&self, unstructured: &mut Unstructured) -> Result<Vec<i32>> {
        match self {
            Self::Value(range) => {
                let value = unstructured.int_in_range(range.clone())?;
                Ok(value
                    .to_le_bytes()
                    .chunks(mem::size_of::<u128>() / mem::size_of::<i32>())
                    .map(|word_bytes| {
                        i32::from_le_bytes(word_bytes.try_into().expect("Chunks are of the exact size."))
                    })
                    .collect())
            }
            _ => todo!("TODO"),
        }
    }
}

impl From<PtrParamDataGenerator> for Ptr {
    fn from(ptr_data: PtrParamDataGenerator) -> Self {
        match ptr_data {
            PtrParamDataGenerator::Value(_) => Ptr::Value,
            PtrParamDataGenerator::HashWithValue { ty, .. } => Ptr::HashWithValue(ty),
            PtrParamDataGenerator::TwoHashesWithValue { ty1, ty2, .. } => {
                Ptr::TwoHashesWithValue(ty1, ty2)
            }
        }
    }
}
