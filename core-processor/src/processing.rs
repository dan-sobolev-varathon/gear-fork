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

use crate::{
    common::{
        ActorExecutionErrorReplyReason, DispatchOutcome, DispatchResult, DispatchResultKind,
        ExecutionError, JournalNote, SystemExecutionError, WasmExecutionContext,
    },
    configs::{BlockConfig, ExecutionSettings},
    context::*,
    executor,
    ext::ProcessorExternalities,
    precharge::SuccessfulDispatchResultKind,
};
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use gear_core::{
    env::Externalities,
    ids::{prelude::*, MessageId, ProgramId},
    message::{ContextSettings, DispatchKind, IncomingDispatch, ReplyMessage, StoredDispatch},
    reservation::GasReservationState,
};
use gear_core_backend::{
    error::{BackendAllocSyscallError, BackendSyscallError, RunFallibleError},
    BackendExternalities,
};
use gear_core_errors::{ErrorReplyReason, SignalCode};

/// Process program & dispatch for it and return journal for updates.
pub fn process<Ext>(
    block_config: &BlockConfig,
    execution_context: ProcessExecutionContext,
    random_data: (Vec<u8>, u32),
) -> Result<Vec<JournalNote>, SystemExecutionError>
where
    Ext: ProcessorExternalities + BackendExternalities + 'static,
    <Ext as Externalities>::AllocError:
        BackendAllocSyscallError<ExtError = Ext::UnrecoverableError>,
    RunFallibleError: From<Ext::FallibleError>,
    <Ext as Externalities>::UnrecoverableError: BackendSyscallError,
{
    use crate::precharge::SuccessfulDispatchResultKind::*;

    let BlockConfig {
        block_info,
        performance_multiplier,
        forbidden_funcs,
        reserve_for,
        gas_multiplier,
        costs,
        existential_deposit,
        mailbox_threshold,
        max_pages,
        outgoing_limit,
        outgoing_bytes_limit,
        ..
    } = block_config.clone();

    let execution_settings = ExecutionSettings {
        block_info,
        performance_multiplier,
        existential_deposit,
        mailbox_threshold,
        max_pages,
        ext_costs: costs.ext,
        lazy_pages_costs: costs.lazy_pages,
        forbidden_funcs,
        reserve_for,
        random_data,
        gas_multiplier,
    };

    let dispatch = execution_context.dispatch;
    let balance = execution_context.balance;
    let program_id = execution_context.program.id;
    let execution_context = WasmExecutionContext {
        gas_counter: execution_context.gas_counter,
        gas_allowance_counter: execution_context.gas_allowance_counter,
        gas_reserver: execution_context.gas_reserver,
        program: execution_context.program,
        memory_size: execution_context.memory_size,
    };

    // Sending fee: double write cost for addition and removal some time soon
    // from queue.
    //
    // Scheduled sending fee: double write cost for addition and removal some time soon
    // from queue and double write cost (addition and removal) for dispatch stash.
    //
    // Waiting fee: triple write cost for addition and removal some time soon
    // from waitlist and enqueuing / sending error reply afterward.
    //
    // Waking fee: double write cost for removal from waitlist
    // and further enqueueing.
    let msg_ctx_settings = ContextSettings {
        sending_fee: costs.write.cost_for(2.into()),
        scheduled_sending_fee: costs.write.cost_for(4.into()),
        waiting_fee: costs.write.cost_for(3.into()),
        waking_fee: costs.write.cost_for(2.into()),
        reservation_fee: costs.write.cost_for(2.into()),
        outgoing_limit,
        outgoing_bytes_limit,
    };

    // TODO: add tests that system reservation is successfully unreserved after
    // actor execution error #3756.

    // Get system reservation context in order to use it if actor execution error occurs.
    let system_reservation_ctx = SystemReservationContext::from_dispatch(&dispatch);

    let exec_result = executor::execute_wasm::<Ext>(
        balance,
        dispatch.clone(),
        execution_context,
        execution_settings,
        msg_ctx_settings,
    )
    .map_err(|err| {
        log::debug!("Wasm execution error: {}", err);
        err
    });

    match exec_result {
        Ok(res) => Ok(match res.kind {
            DispatchResultKind::Trap(reason) => process_execution_error(
                res.dispatch,
                program_id,
                res.gas_amount.burned(),
                res.system_reservation_context,
                ActorExecutionErrorReplyReason::Trap(reason),
            ),
            DispatchResultKind::Success => process_success(Success, res),
            DispatchResultKind::Wait(duration, ref waited_type) => {
                process_success(Wait(duration, waited_type.clone()), res)
            }
            DispatchResultKind::Exit(value_destination) => {
                process_success(Exit(value_destination), res)
            }
            DispatchResultKind::GasAllowanceExceed => {
                process_allowance_exceed(dispatch, program_id, res.gas_amount.burned())
            }
        }),
        Err(ExecutionError::Actor(e)) => Ok(process_execution_error(
            dispatch,
            program_id,
            e.gas_amount.burned(),
            system_reservation_ctx,
            e.reason,
        )),
        Err(ExecutionError::System(e)) => Err(e),
    }
}

enum ProcessErrorCase {
    /// Message is not executable error.
    NonExecutable,
    /// Error is considered as an execution failure.
    ExecutionFailed(ActorExecutionErrorReplyReason),
    /// Message is executable, but it's execution failed due to re-instrumentation.
    ReinstrumentationFailed,
}

impl ProcessErrorCase {
    pub fn to_reason_and_payload(&self) -> (ErrorReplyReason, String) {
        match self {
            ProcessErrorCase::NonExecutable => {
                let reason = ErrorReplyReason::InactiveActor;
                (reason, reason.to_string())
            }
            ProcessErrorCase::ExecutionFailed(reason) => {
                (reason.as_simple().into(), reason.to_string())
            }
            ProcessErrorCase::ReinstrumentationFailed => {
                let err = ErrorReplyReason::ReinstrumentationFailure;
                (err, err.to_string())
            }
        }
    }
}

fn process_error(
    dispatch: IncomingDispatch,
    program_id: ProgramId,
    gas_burned: u64,
    system_reservation_ctx: SystemReservationContext,
    case: ProcessErrorCase,
) -> Vec<JournalNote> {
    let mut journal = Vec::new();

    let message_id = dispatch.id();
    let origin = dispatch.source();
    let value = dispatch.value();

    journal.push(JournalNote::GasBurned {
        message_id,
        amount: gas_burned,
    });

    // We check if value is greater than zero to don't provide
    // no-op journal note.
    //
    // We also check if dispatch had context of previous executions:
    // it's existence shows that we have processed message after
    // being waken, so the value were already transferred in
    // execution, where `gr_wait` was called.
    if dispatch.context().is_none() && value != 0 {
        // Send back value
        journal.push(JournalNote::SendValue {
            from: origin,
            to: None,
            value,
        });
    }

    if let Some(amount) = system_reservation_ctx.current_reservation {
        journal.push(JournalNote::SystemReserveGas { message_id, amount });
    }

    if let ProcessErrorCase::ExecutionFailed(reason) = &case {
        // TODO: consider to handle error reply and init #3701
        if system_reservation_ctx.has_any()
            && !dispatch.is_error_reply()
            && !matches!(dispatch.kind(), DispatchKind::Signal | DispatchKind::Init)
        {
            journal.push(JournalNote::SendSignal {
                message_id,
                destination: program_id,
                code: SignalCode::Execution(reason.as_simple()),
            });
        }
    }

    if system_reservation_ctx.has_any() {
        journal.push(JournalNote::SystemUnreserveGas { message_id });
    }

    if !dispatch.is_reply() && dispatch.kind() != DispatchKind::Signal {
        let (err, err_payload) = case.to_reason_and_payload();

        // Panic is impossible, unless error message is too large or [Payload] max size is too small.
        let err_payload = err_payload
            .into_bytes()
            .try_into()
            .unwrap_or_else(|_| unreachable!("Error message is too large"));

        // # Safety
        //
        // 1. The dispatch.id() has already been checked
        // 2. This reply message is generated by our system
        //
        // So, the message id of this reply message will not be duplicated.
        let dispatch = ReplyMessage::system(dispatch.id(), err_payload, err).into_dispatch(
            program_id,
            dispatch.source(),
            dispatch.id(),
        );

        journal.push(JournalNote::SendDispatch {
            message_id,
            dispatch,
            delay: 0,
            reservation: None,
        });
    }

    let outcome = match case {
        ProcessErrorCase::ExecutionFailed { .. } | ProcessErrorCase::ReinstrumentationFailed => {
            let (_, err_payload) = case.to_reason_and_payload();
            match dispatch.kind() {
                DispatchKind::Init => DispatchOutcome::InitFailure {
                    program_id,
                    origin,
                    reason: err_payload,
                },
                _ => DispatchOutcome::MessageTrap {
                    program_id,
                    trap: err_payload,
                },
            }
        }
        ProcessErrorCase::NonExecutable => DispatchOutcome::NoExecution,
    };

    journal.push(JournalNote::MessageDispatched {
        message_id,
        source: origin,
        outcome,
    });
    journal.push(JournalNote::MessageConsumed(message_id));

    journal
}

/// Helper function for journal creation in trap/error case.
pub fn process_execution_error(
    dispatch: IncomingDispatch,
    program_id: ProgramId,
    gas_burned: u64,
    system_reservation_ctx: SystemReservationContext,
    err: impl Into<ActorExecutionErrorReplyReason>,
) -> Vec<JournalNote> {
    process_error(
        dispatch,
        program_id,
        gas_burned,
        system_reservation_ctx,
        ProcessErrorCase::ExecutionFailed(err.into()),
    )
}

/// Helper function for journal creation in case of re-instrumentation error.
pub fn process_reinstrumentation_error(
    context: ContextChargedForInstrumentation,
) -> Vec<JournalNote> {
    let dispatch = context.data.dispatch;
    let program_id = context.data.destination_id;
    let gas_burned = context.data.gas_counter.burned();
    let system_reservation_ctx = SystemReservationContext::from_dispatch(&dispatch);

    process_error(
        dispatch,
        program_id,
        gas_burned,
        system_reservation_ctx,
        ProcessErrorCase::ReinstrumentationFailed,
    )
}

/// Helper function for journal creation in message no execution case.
pub fn process_non_executable(context: ContextChargedForProgram) -> Vec<JournalNote> {
    let ContextChargedForProgram {
        dispatch,
        gas_counter,
        destination_id,
        ..
    } = context;

    let system_reservation_ctx = SystemReservationContext::from_dispatch(&dispatch);

    process_error(
        dispatch,
        destination_id,
        gas_counter.burned(),
        system_reservation_ctx,
        ProcessErrorCase::NonExecutable,
    )
}

/// Helper function for journal creation in success case
pub fn process_success(
    kind: SuccessfulDispatchResultKind,
    dispatch_result: DispatchResult,
) -> Vec<JournalNote> {
    use crate::precharge::SuccessfulDispatchResultKind::*;

    let DispatchResult {
        dispatch,
        generated_dispatches,
        awakening,
        program_candidates,
        gas_amount,
        gas_reserver,
        system_reservation_context,
        page_update,
        program_id,
        context_store,
        allocations,
        reply_deposits,
        reply_sent,
        ..
    } = dispatch_result;

    let mut journal = Vec::new();

    let message_id = dispatch.id();
    let origin = dispatch.source();
    let value = dispatch.value();

    journal.push(JournalNote::GasBurned {
        message_id,
        amount: gas_amount.burned(),
    });

    if let Some(gas_reserver) = gas_reserver {
        journal.extend(gas_reserver.states().iter().flat_map(
            |(&reservation_id, &state)| match state {
                GasReservationState::Exists { .. } => None,
                GasReservationState::Created {
                    amount, duration, ..
                } => Some(JournalNote::ReserveGas {
                    message_id,
                    reservation_id,
                    program_id,
                    amount,
                    duration,
                }),
                GasReservationState::Removed { expiration } => Some(JournalNote::UnreserveGas {
                    reservation_id,
                    program_id,
                    expiration,
                }),
            },
        ));

        journal.push(JournalNote::UpdateGasReservations {
            program_id,
            reserver: gas_reserver,
        });
    }

    if let Some(amount) = system_reservation_context.current_reservation {
        journal.push(JournalNote::SystemReserveGas { message_id, amount });
    }

    // We check if value is greater than zero to don't provide
    // no-op journal note.
    //
    // We also check if dispatch had context of previous executions:
    // it's existence shows that we have processed message after
    // being waken, so the value were already transferred in
    // execution, where `gr_wait` was called.
    if dispatch.context().is_none() && value != 0 {
        // Send value further
        journal.push(JournalNote::SendValue {
            from: origin,
            to: Some(program_id),
            value,
        });
    }

    // Must be handled before handling generated dispatches.
    for (code_id, candidates) in program_candidates {
        journal.push(JournalNote::StoreNewPrograms {
            program_id,
            code_id,
            candidates,
        });
    }

    // Sending auto-generated reply about success execution.
    if matches!(kind, SuccessfulDispatchResultKind::Success)
        && !reply_sent
        && !dispatch.is_reply()
        && dispatch.kind() != DispatchKind::Signal
    {
        let auto_reply = ReplyMessage::auto(dispatch.id()).into_dispatch(
            program_id,
            dispatch.source(),
            dispatch.id(),
        );

        journal.push(JournalNote::SendDispatch {
            message_id,
            dispatch: auto_reply,
            delay: 0,
            reservation: None,
        });
    }

    for (message_id_sent, amount) in reply_deposits {
        journal.push(JournalNote::ReplyDeposit {
            message_id,
            future_reply_id: MessageId::generate_reply(message_id_sent),
            amount,
        });
    }

    for (dispatch, delay, reservation) in generated_dispatches {
        journal.push(JournalNote::SendDispatch {
            message_id,
            dispatch,
            delay,
            reservation,
        });
    }

    for (awakening_id, delay) in awakening {
        journal.push(JournalNote::WakeMessage {
            message_id,
            program_id,
            awakening_id,
            delay,
        });
    }

    for (page_number, data) in page_update {
        journal.push(JournalNote::UpdatePage {
            program_id,
            page_number,
            data,
        })
    }

    if let Some(allocations) = allocations {
        journal.push(JournalNote::UpdateAllocations {
            program_id,
            allocations,
        });
    }

    let outcome = match kind {
        Wait(duration, waited_type) => {
            journal.push(JournalNote::WaitDispatch {
                dispatch: dispatch.into_stored(program_id, context_store),
                duration,
                waited_type,
            });

            return journal;
        }
        Success => match dispatch.kind() {
            DispatchKind::Init => DispatchOutcome::InitSuccess { program_id },
            _ => DispatchOutcome::Success,
        },
        Exit(value_destination) => {
            journal.push(JournalNote::ExitDispatch {
                id_exited: program_id,
                value_destination,
            });

            DispatchOutcome::Exit { program_id }
        }
    };

    if system_reservation_context.has_any() {
        journal.push(JournalNote::SystemUnreserveGas { message_id });
    }

    journal.push(JournalNote::MessageDispatched {
        message_id,
        source: origin,
        outcome,
    });
    journal.push(JournalNote::MessageConsumed(message_id));
    journal
}

pub fn process_allowance_exceed(
    dispatch: IncomingDispatch,
    program_id: ProgramId,
    gas_burned: u64,
) -> Vec<JournalNote> {
    let mut journal = Vec::with_capacity(1);

    let (kind, message, opt_context) = dispatch.into_parts();

    let dispatch = StoredDispatch::new(kind, message.into_stored(program_id), opt_context);

    journal.push(JournalNote::StopProcessing {
        dispatch,
        gas_burned,
    });

    journal
}
