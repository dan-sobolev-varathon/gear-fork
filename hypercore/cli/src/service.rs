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

use crate::config::Config;
use anyhow::Result;

/// Hypercore service.
pub struct Service {
    db: Box<dyn hypercore_db::Database>,
    network: hypercore_network::Network,
    observer: hypercore_observer::Observer,
}

impl Service {
    pub fn start(config: &Config) -> Result<Self> {
        let db: Box<dyn hypercore_db::Database> = Box::new(hypercore_db::RocksDatabase::open(
            config.database_path.clone(),
        )?);
        let network = hypercore_network::Network::start()?;
        let observer =
            hypercore_observer::Observer::new(config.ethereum_rpc.clone(), db.clone_boxed())?;

        Ok(Self {
            db,
            network,
            observer,
        })
    }
}

#[cfg(test)]
mod tests {

    use super::Service;
    use crate::config::Config;

    #[test]
    fn basics() {
        let service = Service::start(&Config {
            database_path: "/tmp/db".into(),
            ethereum_rpc: "http://localhost:8545".into(),
            key_path: "/tmp/key".into(),
            network_path: "/tmp/net".into(),
        });

        assert!(service.is_ok());
    }
}
