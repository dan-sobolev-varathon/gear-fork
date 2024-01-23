
// This file is part of Gear.

// Copyright (C) 2021-2024 Gear Technologies Inc.
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

//! Autogenerated weights for pallet_gear_voucher
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-12-26, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! CPU: `<UNKNOWN>`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("vara-dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/gear benchmark pallet --chain=vara-dev --steps=50 --repeat=20 --pallet=pallet_gear_voucher --extrinsic=* --execution=wasm --wasm-execution=compiled --heap-pages=4096 --output=./scripts/benchmarking/weights-output/pallet_gear_voucher.rs --template=.maintain/frame-weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::{traits::Get, weights::{constants::RocksDbWeight, Weight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_gear_voucher.
pub trait WeightInfo {
    fn issue() -> Weight;
    fn revoke() -> Weight;
    fn update() -> Weight;
}

/// Weights for pallet_gear_voucher using the Gear node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn issue() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `282`
        //  Estimated: `8245`
        // Minimum execution time: 35_000_000 picoseconds.
        Weight::from_parts(36_000_000, 8245)
            .saturating_add(T::DbWeight::get().reads(3_u64))
            .saturating_add(T::DbWeight::get().writes(4_u64))
    }
    fn revoke() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `542`
        //  Estimated: `10203`
        // Minimum execution time: 31_000_000 picoseconds.
        Weight::from_parts(32_000_000, 10203)
            .saturating_add(T::DbWeight::get().reads(3_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }
    fn update() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `1056`
        //  Estimated: `10717`
        // Minimum execution time: 33_000_000 picoseconds.
        Weight::from_parts(34_000_000, 10717)
            .saturating_add(T::DbWeight::get().reads(3_u64))
            .saturating_add(T::DbWeight::get().writes(3_u64))
    }
}

impl WeightInfo for () {
    fn issue() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `282`
        //  Estimated: `8245`
        // Minimum execution time: 35_000_000 picoseconds.
        Weight::from_parts(36_000_000, 8245)
            .saturating_add(RocksDbWeight::get().reads(3_u64))
            .saturating_add(RocksDbWeight::get().writes(4_u64))
    }
    fn revoke() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `542`
        //  Estimated: `10203`
        // Minimum execution time: 31_000_000 picoseconds.
        Weight::from_parts(32_000_000, 10203)
            .saturating_add(RocksDbWeight::get().reads(3_u64))
            .saturating_add(RocksDbWeight::get().writes(2_u64))
    }
    fn update() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `1056`
        //  Estimated: `10717`
        // Minimum execution time: 33_000_000 picoseconds.
        Weight::from_parts(34_000_000, 10717)
            .saturating_add(RocksDbWeight::get().reads(3_u64))
            .saturating_add(RocksDbWeight::get().writes(3_u64))
    }
}
