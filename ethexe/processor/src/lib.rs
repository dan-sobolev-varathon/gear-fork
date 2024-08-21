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

//! Program's execution service for eGPU.

use anyhow::Result;
use core_processor::common::JournalNote;
use ethexe_common::{
    mirror::Event as MirrorEvent,
    router::{Event as RouterEvent, StateTransition},
    wvara::Event as WVaraEvent,
    BlockEvent,
};
use ethexe_db::{BlockMetaStorage, CodesStorage, Database};
use ethexe_runtime_common::state::{Dispatch, MaybeHash, Storage};
use gear_core::{
    ids::{prelude::CodeIdExt, ActorId, MessageId, ProgramId},
    message::{DispatchKind, Payload},
};
use gprimitives::{CodeId, H256};
use host::InstanceCreator;
use parity_scale_codec::{Decode, Encode};
use std::collections::{BTreeMap, VecDeque};

pub mod host;
mod run;

#[cfg(test)]
mod tests;

pub struct UserMessage {
    id: MessageId,
    kind: DispatchKind,
    source: ActorId,
    payload: Vec<u8>,
    value: u128,
}

pub struct Processor {
    db: Database,
    creator: InstanceCreator,
}

// TODO (breathx): rename outcomes accordingly to events.
/// Local changes that can be committed to the network or local signer.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
pub enum LocalOutcome {
    /// Produced when code with specific id is recorded and available in database.
    CodeApproved(CodeId),

    // TODO: add docs
    CodeRejected(CodeId),

    Transition(StateTransition),
}

/// TODO: consider avoiding re-instantiations on processing events.
/// Maybe impl `struct EventProcessor`.
impl Processor {
    pub fn new(db: Database) -> Result<Self> {
        let creator = InstanceCreator::new(db.clone(), host::runtime())?;
        Ok(Self { db, creator })
    }

    /// Returns some CodeId in case of settlement and new code accepting.
    pub fn handle_new_code(&mut self, original_code: impl AsRef<[u8]>) -> Result<Option<CodeId>> {
        let mut executor = self.creator.instantiate()?;

        let original_code = original_code.as_ref();

        let Some(instrumented_code) = executor.instrument(original_code)? else {
            return Ok(None);
        };

        let code_id = self.db.set_original_code(original_code);

        self.db.set_instrumented_code(
            instrumented_code.instruction_weights_version(),
            code_id,
            instrumented_code,
        );

        Ok(Some(code_id))
    }

    /// Returns bool defining was newly re-instrumented code settled or not.
    pub fn reinstrument_code(&mut self, code_id: CodeId) -> Result<bool> {
        let Some(original_code) = self.db.original_code(code_id) else {
            anyhow::bail!("it's impossible to reinstrument inexistent code");
        };

        let mut executor = self.creator.instantiate()?;

        let Some(instrumented_code) = executor.instrument(&original_code)? else {
            return Ok(false);
        };

        self.db.set_instrumented_code(
            instrumented_code.instruction_weights_version(),
            code_id,
            instrumented_code,
        );

        Ok(true)
    }

    pub fn handle_new_program(&mut self, program_id: ProgramId, code_id: CodeId) -> Result<()> {
        // TODO (breathx): impl key_exists().
        if self.db.original_code(code_id).is_none() {
            anyhow::bail!("code existence should be checked on smart contract side");
        }

        if self.db.program_code_id(program_id).is_some() {
            anyhow::bail!("program duplicates should be checked on smart contract side");
        }

        self.db.set_program_code_id(program_id, code_id);

        Ok(())
    }

    pub fn handle_executable_balance_top_up(
        &mut self,
        state_hash: H256,
        value: u128,
    ) -> Result<H256> {
        let mut state = self
            .db
            .read_state(state_hash)
            .ok_or_else(|| anyhow::anyhow!("program should exist"))?;

        state.executable_balance += value;

        Ok(self.db.write_state(state))
    }

    pub fn handle_user_message(
        &mut self,
        program_hash: H256,
        messages: Vec<UserMessage>,
    ) -> Result<H256> {
        if messages.is_empty() {
            return Ok(program_hash);
        }

        let mut dispatches = Vec::with_capacity(messages.len());

        for message in messages {
            let payload = Payload::try_from(message.payload)
                .map_err(|_| anyhow::anyhow!("payload should be checked on eth side"))?;

            let payload_hash = payload
                .inner()
                .is_empty()
                .then_some(MaybeHash::Empty)
                .unwrap_or_else(|| self.db.write_payload(payload).into());

            let dispatch = Dispatch {
                id: message.id,
                kind: message.kind,
                source: message.source,
                payload_hash,
                value: message.value,
                // TODO: handle replies.
                details: None,
                context: None,
            };

            dispatches.push(dispatch);
        }

        let mut program_state = self
            .db
            .read_state(program_hash)
            .ok_or_else(|| anyhow::anyhow!("program should exist"))?;

        let mut queue = if let MaybeHash::Hash(queue_hash_and_len) = program_state.queue_hash {
            self.db
                .read_queue(queue_hash_and_len.hash)
                .ok_or_else(|| anyhow::anyhow!("queue should exist if hash present"))?
        } else {
            VecDeque::with_capacity(dispatches.len())
        };

        queue.extend(dispatches);

        let queue_hash = self.db.write_queue(queue);

        program_state.queue_hash = MaybeHash::Hash(queue_hash.into());

        Ok(self.db.write_state(program_state))
    }

    pub fn run_on_host(
        &mut self,
        program_id: ProgramId,
        program_state: H256,
    ) -> Result<Vec<JournalNote>> {
        let original_code_id = self.db.program_code_id(program_id).unwrap();

        let maybe_instrumented_code = self
            .db
            .instrumented_code(ethexe_runtime::VERSION, original_code_id);

        let mut executor = self.creator.instantiate()?;

        executor.run(
            program_id,
            original_code_id,
            program_state,
            maybe_instrumented_code,
        )
    }

    // TODO: replace LocalOutcome with Transition struct.
    pub fn run(
        &mut self,
        chain_head: H256,
        programs: &mut BTreeMap<ProgramId, H256>,
    ) -> Result<Vec<LocalOutcome>> {
        self.creator.set_chain_head(chain_head);

        log::debug!("{programs:?}");

        let messages_and_outcomes = run::run(8, self.creator.clone(), programs);

        Ok(messages_and_outcomes.1)
    }

    pub fn process_upload_code(
        &mut self,
        code_id: CodeId,
        code: &[u8],
    ) -> Result<Vec<LocalOutcome>> {
        log::debug!("Processing upload code {code_id:?}");

        if code_id != CodeId::generate(code) || self.handle_new_code(code)?.is_none() {
            Ok(vec![LocalOutcome::CodeRejected(code_id)])
        } else {
            Ok(vec![LocalOutcome::CodeApproved(code_id)])
        }
    }

    pub fn process_block_events(
        &mut self,
        block_hash: H256,
        // TODO (breathx): accept not ref?
        events: &[BlockEvent],
    ) -> Result<Vec<LocalOutcome>> {
        log::debug!("Processing events for {block_hash:?}: {events:?}");

        let mut states = self
            .db
            .block_start_program_states(block_hash)
            .unwrap_or_default();

        for event in events {
            match event {
                BlockEvent::Router(event) => {
                    self.process_router_event(&mut states, event.clone())?;
                }
                BlockEvent::Mirror { address, event } => {
                    self.process_mirror_event(&mut states, *address, event.clone())?;
                }
                BlockEvent::WVara(event) => {
                    self.process_wvara_event(&mut states, event.clone())?;
                }
            }
        }

        let outcomes = self.run(block_hash, &mut states)?;

        self.db.set_block_end_program_states(block_hash, states);

        Ok(outcomes)
    }

    fn process_router_event(
        &mut self,
        states: &mut BTreeMap<ProgramId, H256>,
        event: RouterEvent,
    ) -> Result<()> {
        match event {
            RouterEvent::ProgramCreated { actor_id, code_id } => {
                self.handle_new_program(actor_id, code_id)?;

                states.insert(actor_id, H256::zero());
            }
            _ => {
                log::debug!("Handling for router event {event:?} is not yet implemented; noop");
            }
        };

        Ok(())
    }

    fn process_mirror_event(
        &mut self,
        states: &mut BTreeMap<ProgramId, H256>,
        actor_id: ProgramId,
        event: MirrorEvent,
    ) -> Result<()> {
        let Some(&state_hash) = states.get(&actor_id) else {
            log::debug!("Received event from unrecognized mirror ({actor_id}): {event:?}");

            return Ok(());
        };

        let new_state_hash = match event {
            MirrorEvent::ExecutableBalanceTopUpRequested { value } => {
                self.handle_executable_balance_top_up(state_hash, value)?
            }
            MirrorEvent::MessageQueueingRequested {
                id,
                source,
                payload,
                value,
            } => {
                let kind = if state_hash.is_zero() {
                    DispatchKind::Init
                } else {
                    DispatchKind::Handle
                };

                self.handle_user_message(
                    state_hash,
                    vec![UserMessage {
                        id,
                        kind,
                        source,
                        payload,
                        value,
                    }],
                )?
            }
            _ => {
                log::debug!("Handler for this event isn't yet implemented: {event:?}");

                return Ok(());
            }
        };

        states.insert(actor_id, new_state_hash);

        Ok(())
    }

    fn process_wvara_event(
        &mut self,
        _states: &mut BTreeMap<ProgramId, H256>,
        event: WVaraEvent,
    ) -> Result<()> {
        log::debug!("Handler for this event isn't yet implemented: {event:?}");

        Ok(())
    }
}
