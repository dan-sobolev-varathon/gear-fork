// This file is part of Gear.

// Copyright (C) 2022-2024 Gear Technologies Inc.
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

use crate::client::{Backend, Message, TxResult};
use anyhow::Result;
use gear_core::ids::ProgramId;
use gprimitives::MessageId;

/// Gear program instance
pub struct Program<T: Backend> {
    /// Id of this program
    pub(crate) id: ProgramId,
    /// Backend pointer
    pub(crate) backend: T,
}

impl<T: Backend> Program<T> {
    /// Send message to the program
    pub async fn send(&self, message: impl Into<Message>) -> Result<TxResult<MessageId>> {
        self.backend.send(self.id, message.into()).await
    }
}
