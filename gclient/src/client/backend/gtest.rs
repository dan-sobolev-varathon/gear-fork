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

use crate::client::{Backend, Code, Message, Program, TxResult};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use gear_core::{ids::ProgramId, message::UserStoredMessage};
use gprimitives::{ActorId, MessageId};
use gsdk::metadata::runtime_types::gear_common::storage::primitives::Interval;
use gtest::System;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, SystemTime},
};
use tokio::{
    sync::{
        mpsc::{self, Sender},
        Mutex,
    },
    task::{JoinHandle, LocalSet},
};

/// gear general client gtest backend
#[derive(Clone)]
pub struct GTest {
    tx: Sender<Request>,
    results: Arc<Mutex<HashMap<usize, Response>>>,
    timeout: Duration,
    _handle: Arc<JoinHandle<()>>,
    nonce: Arc<AtomicUsize>,
}

impl GTest {
    /// New gtest instance
    ///
    /// gtest system is running in local thread in `GTest`,
    /// the first argument is for channel size, and the second
    /// one is the timeout for waiting response.
    ///
    /// We recommend you to use `GTest::default` instead of this
    /// method if the paramters bother you.
    pub fn new(size: usize, timeout: Duration) -> Self {
        let local = LocalSet::new();
        let results = Arc::new(Mutex::new(HashMap::new()));
        let (tx, mut rx) = mpsc::channel::<Request>(size);

        let cloned = results.clone();
        let handle = local.spawn_local(async move {
            let system = System::new();
            while let Some(tx) = rx.recv().await {
                let (result, nounce) = match tx {
                    Request::Deploy {
                        nonce,
                        code,
                        message,
                        signer,
                    } => (handle::deploy(&system, code, message, signer), nonce),
                    Request::Send {
                        nonce,
                        prog,
                        message,
                        signer,
                    } => (handle::send(&system, prog, message, signer), nonce),
                    Request::Program { nonce, id } => (handle::prog(&system, id), nonce),
                };

                cloned.lock().await.insert(nounce, result);
            }
        });

        Self {
            tx,
            results,
            timeout,
            nonce: Arc::new(AtomicUsize::new(0)),
            _handle: Arc::new(handle),
        }
    }

    /// Get gtest result from nonce.
    async fn resp(&self, nonce: usize) -> Result<Response> {
        let now = SystemTime::now();

        loop {
            if now.elapsed()? > self.timeout {
                return Err(anyhow!("gtest: Transaction timed out!"));
            }

            if let Some(resp) = self.results.lock().await.remove(&nonce) {
                return Ok(resp);
            }
        }
    }
}

#[async_trait]
impl Backend for GTest {
    async fn program(&self, id: ProgramId) -> Result<Program<Self>> {
        let nonce = self.nonce.load(Ordering::SeqCst);
        self.tx.send(Request::Program { nonce, id }).await?;

        let result = self.resp(nonce).await?;
        let Response::Program(result) = result else {
            return Err(anyhow!(
                "Response is not matched with deploy request, {result:?}"
            ));
        };

        Ok(Program {
            id: result.ok_or(anyhow!("Program {id} not found"))?,
            backend: self.clone(),
        })
    }

    async fn deploy<M>(&self, code: impl Code, message: M) -> Result<TxResult<Program<Self>>>
    where
        M: Into<Message> + Send,
    {
        let nonce = self.nonce.load(Ordering::SeqCst);
        self.tx
            .send(Request::Deploy {
                nonce,
                code: code.wasm()?,
                message: message.into(),
                signer: Default::default(),
            })
            .await?;

        let result = self.resp(nonce).await?;
        let Response::Deploy(result) = result else {
            return Err(anyhow!(
                "Response is not matched with deploy request, {result:?}"
            ));
        };

        Ok(TxResult {
            result: Program {
                id: result.result,
                backend: self.clone(),
            },
            logs: result.logs,
        })
    }

    async fn send<M>(&self, id: ProgramId, message: M) -> Result<TxResult<MessageId>>
    where
        M: Into<Message> + Send,
    {
        let nonce = self.nonce.load(Ordering::SeqCst);
        self.tx
            .send(Request::Send {
                nonce,
                prog: id.into(),
                message: message.into(),
                signer: Default::default(),
            })
            .await?;

        let result = self.resp(nonce).await?;
        let Response::Send(result) = result else {
            return Err(anyhow!(
                "Response is not matched with send request, {result:?}"
            ));
        };

        Ok(result)
    }

    async fn message(&self, _mid: MessageId) -> Result<Option<(UserStoredMessage, Interval<u32>)>> {
        Err(anyhow!(
            "gtest backend currently doesn't support this method"
        ))
    }
}

impl Default for GTest {
    fn default() -> Self {
        Self::new(10, Duration::from_millis(500))
    }
}

/// GTest requests
pub enum Request {
    Deploy {
        nonce: usize,
        code: Vec<u8>,
        message: Message,
        signer: ActorId,
    },
    Send {
        nonce: usize,
        prog: ActorId,
        message: Message,
        signer: ActorId,
    },
    Program {
        nonce: usize,
        id: ProgramId,
    },
}

/// GTest responses
#[derive(Debug, Clone)]
pub enum Response {
    Deploy(TxResult<ActorId>),
    Send(TxResult<MessageId>),
    Program(Option<ActorId>),
}

/// gtest handles
pub(crate) mod handle {
    use crate::client::{backend::gtest::Response, Message, TxResult};
    use gear_core::{
        buffer::LimitedVec,
        ids::{prelude::CodeIdExt, ProgramId},
        message::{ReplyDetails, UserMessage},
    };
    use gprimitives::{ActorId, CodeId};
    use gtest::{CoreLog, Program, System};

    /// Return back program id if program exists
    pub fn prog(system: &System, prog: ProgramId) -> Response {
        Response::Program(system.get_program(prog).map(|p| p.id()))
    }

    /// Deploy program via gtest
    pub fn deploy(system: &System, code: Vec<u8>, message: Message, signer: ActorId) -> Response {
        let id = CodeId::generate(&code);
        let prog = Program::from_binary_with_id(system, code, &id.into_bytes());
        let r = prog.send_bytes(signer, message.payload);

        Response::Deploy(TxResult {
            result: prog.id(),
            logs: map_logs(r.log()),
        })
    }

    /// Send message via gtest
    pub fn send(system: &System, prog: ActorId, message: Message, signer: ActorId) -> Response {
        let prog = system.get_program(prog).unwrap();
        let r = prog.send(signer, message.payload);

        Response::Send(TxResult {
            result: r.sent_message_id(),
            logs: map_logs(r.log()),
        })
    }

    fn map_logs(logs: &[CoreLog]) -> Vec<UserMessage> {
        logs.into_iter()
            .map(|l| {
                UserMessage::new(
                    l.id(),
                    l.source(),
                    l.destination(),
                    LimitedVec::try_from(l.payload().to_vec()).unwrap_or_default(),
                    Default::default(),
                    l.reply_code()
                        .zip(l.reply_to())
                        .map(|(code, to)| ReplyDetails::new(to, code)),
                )
            })
            .collect()
    }
}
