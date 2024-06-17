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

use crate::{JoinError, JoinHandle, JoinResult, TASKS_AMOUNT};
use futures_executor::ThreadPool;
use gear_tasks_runtime_api::GearTasksApi;
use sc_client_api::UsageProvider;
use sp_api::{ApiExt, ProvideRuntimeApi};
use std::{
    collections::HashMap,
    marker::PhantomData,
    sync::{mpsc, Arc, OnceLock},
};

static RUNNER_TX: OnceLock<mpsc::Sender<TaskInfo>> = OnceLock::new();

struct TaskInfo {
    pub func_ref: u64,
    pub payload: Vec<u8>,
    pub rx: mpsc::SyncSender<JoinResult>,
}

sp_externalities::decl_extension! {
    pub(crate) struct GearTasksContextExt;
}

pub struct GearTasksRunner<RA, Block: sp_api::BlockT> {
    runtime_api_provider: Arc<RA>,
    rx: mpsc::Receiver<TaskInfo>,
    thread_pool: ThreadPool,
    _block: PhantomData<Block>,
}

impl<RA, Block> GearTasksRunner<RA, Block>
where
    RA: ProvideRuntimeApi<Block> + UsageProvider<Block> + Send + Sync + 'static,
    RA::Api: GearTasksApi<Block>,
    Block: sp_api::BlockT,
{
    pub fn new(client: Arc<RA>) -> Self {
        let (tx, rx) = mpsc::channel();
        let _tx = RUNNER_TX.get_or_init(move || tx);

        log::error!("TX inited");

        Self {
            runtime_api_provider: client,
            rx,
            thread_pool: ThreadPool::builder()
                .pool_size(TASKS_AMOUNT)
                .name_prefix("gear-tasks-")
                .create()
                .expect("Thread pool creation failed"),
            _block: PhantomData,
        }
    }

    pub async fn run(self) {
        log::error!("RUN started");

        for TaskInfo {
            func_ref,
            payload,
            rx,
        } in self.rx
        {
            let client = self.runtime_api_provider.clone();
            self.thread_pool.spawn_ok(async move {
                let mut runtime_api = client.runtime_api();
                runtime_api.register_extension(GearTasksContextExt);
                let block_hash = client.usage_info().chain.best_hash;

                let res = runtime_api
                    .execute_task(block_hash, func_ref, payload)
                    .map_err(|e| JoinError::RuntimeApi(e.to_string()));

                rx.send(res)
                    .expect("`TaskSpawner` dropped before task completion and `join()` on it")
            });
        }
    }
}

sp_externalities::decl_extension! {
    #[derive(Default)]
    pub struct TaskSpawnerExt(TaskSpawner);
}

#[derive(Default)]
pub struct TaskSpawner {
    counter: u64,
    tasks: HashMap<u64, mpsc::Receiver<JoinResult>>,
}

impl TaskSpawner {
    pub(crate) fn spawn(&mut self, func_ref: u64, payload: Vec<u8>) -> JoinHandle {
        let handle = self.counter;
        self.counter += 1;

        let (rx, tx) = mpsc::sync_channel(1);

        let runner_tx = RUNNER_TX.get().expect("`GearTasksRunner` is not spawned");
        runner_tx
            .send(TaskInfo {
                func_ref,
                payload,
                rx,
            })
            .unwrap();

        self.tasks.insert(handle, tx);
        JoinHandle { inner: handle }
    }

    pub(crate) fn join(&mut self, handle: JoinHandle) -> JoinResult {
        let tx = self
            .tasks
            .remove(&handle.inner)
            .expect("`JoinHandle` is duplicated so task not found");
        tx.recv()
            .expect("Sender has been disconnected which means thread was somehow terminated")
    }
}
