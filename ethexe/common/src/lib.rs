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

//! ethexe common types and traits.

#![no_std]

extern crate alloc;

use gprimitives::ActorId;
use parity_scale_codec::{Decode, Encode};

pub mod db;
pub mod mirror;
pub mod router;
pub mod wvara;

#[derive(Clone, Debug, Encode, Decode)]
pub enum BlockEvent {
    Router(router::Event),
    Mirror {
        address: ActorId,
        event: mirror::Event,
    },
    WVara(wvara::Event),
}

impl BlockEvent {
    pub fn mirror(address: ActorId, event: mirror::Event) -> Self {
        Self::Mirror { address, event }
    }
}

impl From<router::Event> for BlockEvent {
    fn from(value: router::Event) -> Self {
        Self::Router(value)
    }
}

impl From<wvara::Event> for BlockEvent {
    fn from(value: wvara::Event) -> Self {
        Self::WVara(value)
    }
}
