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

#![allow(clippy::items_after_test_module)]

mod gear_calls;
mod runtime;
#[cfg(test)]
mod tests;
mod utils;

use crate::utils::default_fuzzing_config;
use arbitrary::{Arbitrary, Error, Result, Unstructured};
use frame_support::pallet_prelude::DispatchResultWithPostInfo;
use gear_call_gen::{ClaimValueArgs, GearCall, SendMessageArgs, SendReplyArgs, UploadProgramArgs};
use gear_calls::GearCalls;
use gear_core::ids::ProgramId;
use gear_runtime::{AccountId, Gear, RuntimeOrigin};
use runtime::*;
use sha1::*;
use std::{fmt::Debug, ops::RangeInclusive};
use utils::default_generator_set;

/// This is a wrapper over random bytes provided from fuzzer.
///
/// It's main purpose is to be a mock implementor of `Debug`.
/// For more info see `Debug` impl.
pub struct RuntimeFuzzerInput<'a>(&'a [u8]);

impl<'a> Arbitrary<'a> for RuntimeFuzzerInput<'a> {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let data = u.peek_bytes(u.len()).ok_or(Error::NotEnoughData)?;

        Ok(Self(data))
    }
}

/// That's done because when fuzzer finds a crash it prints a [`Debug`] string of the crashing input.
/// Fuzzer constructs from the input an array of [`GearCall`] with pretty large codes and payloads,
/// therefore to avoid printing huge amount of data we do a mock implementation of [`Debug`].
impl Debug for RuntimeFuzzerInput<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("RuntimeFuzzerInput")
            .field(&"Mock `Debug` impl")
            .finish()
    }
}

/// Contains some general configs for fuzzer.
pub(crate) struct FuzzingConfig {
    initial_sender_balance: RangeInclusive<u128>,
    allow_exceed_sender_balance: bool,
}

/// Runs all the fuzz testing internal machinery.
pub fn run(RuntimeFuzzerInput(data): RuntimeFuzzerInput<'_>) -> Result<()> {
    run_impl(data).map(|_| ())
}

fn run_impl(data: &[u8]) -> Result<sp_io::TestExternalities> {
    log::trace!(
        "New GearCalls generation: random data received {}",
        data.len()
    );
    let test_input_id = get_sha1_string(data);
    log::trace!("Generating GearCalls from corpus - {}", test_input_id);

    let fuzzing_config = default_fuzzing_config();

    let sender = runtime::account(runtime::alice());
    let sender_prog_id = ProgramId::from(*<AccountId as AsRef<[u8; 32]>>::as_ref(&sender));

    let mut unstructured = Unstructured::new(data);

    let initial_sender_balance =
        unstructured.int_in_range(fuzzing_config.initial_sender_balance.clone())?;

    let generators = default_generator_set(test_input_id);
    let gear_calls = GearCalls::new(unstructured, generators, vec![sender_prog_id])?;

    let mut test_ext = new_test_ext();
    test_ext.execute_with(|| -> Result<()> {
        // Set balance of the `sender`.
        {
            set_account_balance(sender.clone(), initial_sender_balance)
                .unwrap_or_else(|e| unreachable!("Balance update failed: {e:?}"));
            log::info!(
                "Current balance of the sender - {}",
                get_account_balance(&sender)
            );
        }

        for gear_call in gear_calls {
            let gear_call = gear_call?;
            let call_res = execute_gear_call(sender.clone(), gear_call, &fuzzing_config);
            log::info!("Extrinsic result: {call_res:?}");
            // Run task and message queues with max possible gas limit.
            run_to_next_block();
        }

        Ok(())
    })?;

    Ok(test_ext)
}

fn execute_gear_call(
    sender: AccountId,
    call: GearCall,
    fuzzing_config: &FuzzingConfig,
) -> DispatchResultWithPostInfo {
    let allowed_to_spend_value = if fuzzing_config.allow_exceed_sender_balance {
        u128::MAX
    } else {
        get_account_balance(&sender).saturating_sub(block_gas_cost())
    };

    match call {
        GearCall::UploadProgram(args) => {
            let UploadProgramArgs((code, salt, payload, gas_limit, value)) = args;

            let value = allowed_to_spend_value.min(value);

            Gear::upload_program(
                RuntimeOrigin::signed(sender),
                code,
                salt,
                payload,
                gas_limit,
                value,
            )
        }
        GearCall::SendMessage(args) => {
            let SendMessageArgs((destination, payload, gas_limit, value, prepaid)) = args;

            let value = allowed_to_spend_value.min(value);

            Gear::send_message(
                RuntimeOrigin::signed(sender),
                destination,
                payload,
                gas_limit,
                value,
                prepaid,
            )
        }
        GearCall::SendReply(args) => {
            let SendReplyArgs((message_id, payload, gas_limit, value, prepaid)) = args;

            let value = allowed_to_spend_value.min(value);

            Gear::send_reply(
                RuntimeOrigin::signed(sender),
                message_id,
                payload,
                gas_limit,
                value,
                prepaid,
            )
        }
        GearCall::ClaimValue(args) => {
            let ClaimValueArgs(message_id) = args;
            Gear::claim_value(RuntimeOrigin::signed(sender), message_id)
        }
        _ => unimplemented!("Unsupported currently."),
    }
}

fn get_sha1_string(input: &[u8]) -> String {
    let mut hasher = sha1::Sha1::new();
    hasher.update(input);

    hex::encode(hasher.finalize())
}
