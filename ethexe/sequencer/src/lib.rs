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

//! Sequencer for ethexe.

pub mod agro;

use agro::{AggregatedCommitments, MultisignedCommitmentDigests, MultisignedCommitments};
use anyhow::{anyhow, Result};
use ethexe_common::{BlockCommitment, CodeCommitment};
use ethexe_ethereum::Ethereum;
use ethexe_observer::Event;
use ethexe_signer::{Address, AsDigest, Digest, PublicKey, Signature, Signer};
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    ops::Not,
};
use tokio::sync::watch;

pub struct Sequencer {
    key: PublicKey,
    ethereum: Ethereum,

    validators: HashSet<Address>,
    threshold: u64,

    code_commitments: CommitmentsMap<CodeCommitment>,
    block_commitments: CommitmentsMap<BlockCommitment>,

    codes_candidate: Option<MultisignedCommitmentDigests>,
    blocks_candidate: Option<MultisignedCommitmentDigests>,

    status: SequencerStatus,
    status_sender: watch::Sender<SequencerStatus>,
}

pub struct Config {
    pub ethereum_rpc: String,
    pub sign_tx_public: PublicKey,
    pub router_address: Address,
    pub validators: Vec<Address>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SequencerStatus {
    pub aggregated_commitments: u64,
    pub submitted_code_commitments: u64,
    pub submitted_block_commitments: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommitmentAndOrigins<C> {
    commitment: C,
    origins: BTreeSet<Address>,
}

type CommitmentsMap<C> = BTreeMap<Digest, CommitmentAndOrigins<C>>;

impl Sequencer {
    pub async fn new(config: &Config, signer: Signer) -> Result<Self> {
        let (status_sender, _status_receiver) = watch::channel(SequencerStatus::default());
        Ok(Sequencer {
            key: config.sign_tx_public,
            ethereum: Ethereum::new(
                &config.ethereum_rpc,
                config.router_address,
                signer,
                config.sign_tx_public.to_address(),
            )
            .await?,
            validators: config.validators.iter().cloned().collect(),
            threshold: 1,
            code_commitments: Default::default(),
            block_commitments: Default::default(),
            codes_candidate: Default::default(),
            blocks_candidate: Default::default(),
            status: Default::default(),
            status_sender,
        })
    }

    // This function should never block.
    pub fn process_observer_event(&mut self, event: &Event) -> Result<()> {
        if let Event::Block(data) = event {
            log::debug!("Receive block {:?}", data.block_hash);

            self.update_status(|status| {
                *status = SequencerStatus::default();
            });
        }

        // Presently, sequencer resets candidates for each observer event,
        // because each observer event resets ethexe service rounds.
        self.codes_candidate = None;
        self.blocks_candidate = None;

        Ok(())
    }

    pub fn process_collected_commitments(&mut self) -> Result<()> {
        if self.codes_candidate.is_some() || self.blocks_candidate.is_some() {
            return Err(anyhow!("Previous commitments candidate are not submitted"));
        }

        Self::set_candidate_commitments(
            &self.code_commitments,
            &mut self.codes_candidate,
            self.threshold,
        );

        Self::set_candidate_commitments(
            &self.block_commitments,
            &mut self.blocks_candidate,
            self.threshold,
        );

        Ok(())
    }

    pub async fn submit_multisigned_commitments(&mut self) -> Result<()> {
        let mut codes_future = None;
        let mut blocks_future = None;
        let mut code_commitments_len = 0;
        let mut block_commitments_len = 0;

        let codes_candidate = Self::process_multisigned_candidate(
            &mut self.codes_candidate,
            &mut self.code_commitments,
            self.threshold,
        );

        let blocks_candidate = Self::process_multisigned_candidate(
            &mut self.blocks_candidate,
            &mut self.block_commitments,
            self.threshold,
        );

        if let Some(candidate) = codes_candidate {
            code_commitments_len = candidate.commitments().len() as u64;
            log::debug!("Collected {code_commitments_len} code commitments. Trying to submit...");

            codes_future = Some(self.submit_codes_commitment(candidate));
        };

        if let Some(candidate) = blocks_candidate {
            block_commitments_len = candidate.commitments().len() as u64;
            log::debug!("Collected {block_commitments_len} block commitments. Trying to submit...",);

            blocks_future = Some(self.submit_block_commitments(candidate));
        };

        match (codes_future, blocks_future) {
            (Some(codes_future), Some(transitions_future)) => {
                let (codes_tx, transitions_tx) = futures::join!(codes_future, transitions_future);
                codes_tx?;
                transitions_tx?;
            }
            (Some(codes_future), None) => codes_future.await?,
            (None, Some(transitions_future)) => transitions_future.await?,
            (None, None) => {}
        }

        self.update_status(|status| {
            status.submitted_code_commitments += code_commitments_len;
            status.submitted_block_commitments += block_commitments_len;
        });

        Ok(())
    }

    pub fn receive_code_commitments(
        &mut self,
        aggregated: AggregatedCommitments<CodeCommitment>,
    ) -> Result<()> {
        Self::receive_commitments(
            aggregated,
            &self.validators,
            self.ethereum.router().address(),
            &mut self.code_commitments,
        )
    }

    pub fn receive_block_commitments(
        &mut self,
        aggregated: AggregatedCommitments<BlockCommitment>,
    ) -> Result<()> {
        Self::receive_commitments(
            aggregated,
            &self.validators,
            self.ethereum.router().address(),
            &mut self.block_commitments,
        )
    }

    pub fn receive_codes_signature(&mut self, digest: Digest, signature: Signature) -> Result<()> {
        Self::receive_signature(
            digest,
            signature,
            &self.validators,
            self.ethereum.router().address(),
            self.codes_candidate.as_mut(),
        )
    }

    pub fn receive_blocks_signature(&mut self, digest: Digest, signature: Signature) -> Result<()> {
        Self::receive_signature(
            digest,
            signature,
            &self.validators,
            self.ethereum.router().address(),
            self.blocks_candidate.as_mut(),
        )
    }

    pub fn address(&self) -> Address {
        self.key.to_address()
    }

    pub fn get_status_receiver(&self) -> watch::Receiver<SequencerStatus> {
        self.status_sender.subscribe()
    }

    pub fn get_candidate_code_commitments(&self) -> impl Iterator<Item = &CodeCommitment> + '_ {
        Self::get_candidate_commitments(&self.codes_candidate, &self.code_commitments)
    }

    pub fn get_candidate_block_commitments(&self) -> impl Iterator<Item = &BlockCommitment> + '_ {
        Self::get_candidate_commitments(&self.blocks_candidate, &self.block_commitments)
    }

    fn get_candidate_commitments<'a, C>(
        candidate: &'a Option<MultisignedCommitmentDigests>,
        commitments: &'a CommitmentsMap<C>,
    ) -> impl Iterator<Item = &'a C> + 'a {
        candidate
            .iter()
            .flat_map(|candidate| candidate.digests().iter())
            .map(|digest| {
                commitments
                    .get(digest)
                    .map(|c| &c.commitment)
                    .unwrap_or_else(|| {
                        unreachable!("Guarantied by `Sequencer` implementation to be in the map")
                    })
            })
    }

    fn set_candidate_commitments<C: AsDigest>(
        commitments: &CommitmentsMap<C>,
        candidate: &mut Option<MultisignedCommitmentDigests>,
        threshold: u64,
    ) {
        let suitable_digests: BTreeSet<_> = commitments
            .iter()
            .filter_map(|(digest, c)| (c.origins.len() as u64 >= threshold).then_some(*digest))
            .collect();

        if suitable_digests.is_empty() {
            return;
        }

        *candidate = Some(MultisignedCommitmentDigests::new(suitable_digests));
    }

    fn process_multisigned_candidate<C: AsDigest>(
        candidate: &mut Option<MultisignedCommitmentDigests>,
        commitments: &mut CommitmentsMap<C>,
        threshold: u64,
    ) -> Option<MultisignedCommitments<C>> {
        if candidate
            .as_ref()
            .map(|c| threshold > c.signatures().len() as u64)
            .unwrap_or(true)
        {
            return None;
        }

        let candidate = candidate.take()?;
        let multisigned = MultisignedCommitments::from_multisigned_digests(candidate, |digest| {
            commitments
                .remove(&digest)
                .map(|c| c.commitment)
                .unwrap_or_else(|| {
                    unreachable!("Guarantied by `Sequencer` implementation to be in the map");
                })
        });

        Some(multisigned)
    }

    async fn submit_codes_commitment(
        &self,
        multisigned: MultisignedCommitments<CodeCommitment>,
    ) -> Result<()> {
        let (codes, signatures) = multisigned.into_parts();
        let (origins, signatures): (Vec<_>, _) = signatures.into_iter().unzip();

        log::debug!("Code commitments to submit: {codes:?}, signed by: {origins:?}",);

        let router = self.ethereum.router();
        if let Err(e) = router.commit_codes(codes, signatures).await {
            // TODO: return error?
            log::error!("Failed to commit code ids: {e}");
        }

        Ok(())
    }

    async fn submit_block_commitments(
        &self,
        multisigned: MultisignedCommitments<BlockCommitment>,
    ) -> Result<()> {
        let (blocks, signatures) = multisigned.into_parts();
        let (origins, signatures): (Vec<_>, _) = signatures.into_iter().unzip();

        log::debug!("Block commitments to submit: {blocks:?}, signed by: {origins:?}",);

        let router = self.ethereum.router();
        match router.commit_blocks(blocks, signatures).await {
            Err(e) => {
                // TODO: return error?
                log::error!("Failed to commit transitions: {e}");
            }
            Ok(tx_hash) => {
                log::info!(
                    "Blocks commitment transaction {tx_hash} was added to the pool successfully"
                );
            }
        }

        Ok(())
    }

    fn receive_commitments<C: AsDigest>(
        aggregated: AggregatedCommitments<C>,
        validators: &HashSet<Address>,
        router_address: Address,
        commitments_storage: &mut CommitmentsMap<C>,
    ) -> Result<()> {
        let origin = aggregated.recover(router_address)?;

        if validators.contains(&origin).not() {
            return Err(anyhow!("Unknown validator {origin} or invalid signature"));
        }

        for commitment in aggregated.commitments {
            commitments_storage
                .entry(commitment.as_digest())
                .or_insert_with(|| CommitmentAndOrigins {
                    commitment,
                    origins: Default::default(),
                })
                .origins
                .insert(origin);
        }

        Ok(())
    }

    fn receive_signature(
        commitments_digest: Digest,
        signature: Signature,
        validators: &HashSet<Address>,
        router_address: Address,
        candidate: Option<&mut MultisignedCommitmentDigests>,
    ) -> Result<()> {
        let Some(candidate) = candidate else {
            return Err(anyhow!("No candidate found"));
        };

        candidate.append_signature_with_check(
            commitments_digest,
            signature,
            router_address,
            |origin| {
                validators
                    .contains(&origin)
                    .then_some(())
                    .ok_or_else(|| anyhow!("Unknown validator {origin} or invalid signature"))
            },
        )
    }

    fn update_status<F>(&mut self, update_fn: F)
    where
        F: FnOnce(&mut SequencerStatus),
    {
        let mut status = self.status;
        update_fn(&mut status);
        let _ = self.status_sender.send_replace(status);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Ok;
    use ethexe_signer::PrivateKey;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct TestComm([u8; 2]);

    impl AsDigest for TestComm {
        fn as_digest(&self) -> Digest {
            self.0.as_digest()
        }
    }

    #[test]
    fn test_receive_signature() {
        let signer = Signer::tmp();

        let router_address = Address([1; 20]);

        let validators_private_keys = [PrivateKey([1; 32]), PrivateKey([2; 32])];
        let validators: HashSet<_> = validators_private_keys
            .iter()
            .cloned()
            .map(|key| signer.add_key(key).unwrap().to_address())
            .collect();

        let validator1_private_key = validators_private_keys[0];
        let validator1_pub_key = PublicKey::from(validator1_private_key);
        let validator1 = validator1_pub_key.to_address();

        let commitments = [TestComm([0, 1]), TestComm([2, 3])];
        let commitments_digest = commitments.as_digest();
        let signature = agro::sign_commitments_digest(
            commitments_digest,
            &signer,
            validator1_pub_key,
            router_address,
        )
        .unwrap();

        Sequencer::receive_signature(
            commitments_digest,
            signature,
            &validators,
            router_address,
            None,
        )
        .expect_err("No candidate is provided");

        let mut signatures: BTreeMap<_, _> = Default::default();
        let digests: BTreeSet<_> = commitments.iter().map(AsDigest::as_digest).collect();
        let expected_digests: Vec<_> = digests.iter().cloned().collect();
        let mut candidate = MultisignedCommitmentDigests::new(digests);

        Sequencer::receive_signature(
            Digest::from([1; 32]),
            signature,
            &validators,
            router_address,
            Some(&mut candidate),
        )
        .expect_err("Incorrect digest has been provided");

        Sequencer::receive_signature(
            commitments_digest,
            Signature::create_for_digest(validator1_private_key, Digest::from([1; 32])).unwrap(),
            &validators,
            router_address,
            Some(&mut candidate),
        )
        .expect_err("Signature verification must fail");

        Sequencer::receive_signature(
            commitments_digest,
            signature,
            &validators,
            router_address,
            Some(&mut candidate),
        )
        .unwrap();

        signatures.insert(validator1, signature);
        assert_eq!(expected_digests.as_slice(), candidate.digests());
        assert_eq!(&signatures, candidate.signatures());

        let validator2_private_key = validators_private_keys[1];
        let validator2_pub_key = PublicKey::from(validator2_private_key);
        let validator2 = validator2_pub_key.to_address();

        let signature = agro::sign_commitments_digest(
            commitments_digest,
            &signer,
            validator2_pub_key,
            router_address,
        )
        .unwrap();

        Sequencer::receive_signature(
            commitments_digest,
            signature,
            &validators,
            router_address,
            Some(&mut candidate),
        )
        .unwrap();

        signatures.insert(validator2, signature);
        assert_eq!(expected_digests.as_slice(), candidate.digests());
        assert_eq!(&signatures, candidate.signatures());
    }

    #[test]
    fn test_receive_commitments() {
        let signer = Signer::tmp();

        let router_address = Address([1; 20]);

        let validators_private_keys = [PrivateKey([1; 32]), PrivateKey([2; 32])];
        let validators: HashSet<_> = validators_private_keys
            .iter()
            .cloned()
            .map(|key| signer.add_key(key).unwrap().to_address())
            .collect();

        let validator1_private_key = validators_private_keys[0];
        let validator1_pub_key = PublicKey::from(validator1_private_key);
        let validator1 = validator1_pub_key.to_address();

        let commitments = [TestComm([0, 1]), TestComm([2, 3])];
        let aggregated = AggregatedCommitments::aggregate_commitments(
            commitments.to_vec(),
            &signer,
            validator1_pub_key,
            router_address,
        )
        .unwrap();

        let mut expected_commitments_storage = CommitmentsMap::new();
        let mut commitments_storage = CommitmentsMap::new();

        let private_key = PrivateKey([3; 32]);
        let pub_key = signer.add_key(private_key).unwrap();
        let incorrect_aggregated = AggregatedCommitments::aggregate_commitments(
            commitments.to_vec(),
            &signer,
            pub_key,
            router_address,
        )
        .unwrap();
        Sequencer::receive_commitments(
            incorrect_aggregated,
            &validators,
            router_address,
            &mut commitments_storage,
        )
        .expect_err("Signature verification must fail");

        Sequencer::receive_commitments(
            aggregated.clone(),
            &validators,
            router_address,
            &mut commitments_storage,
        )
        .unwrap();

        expected_commitments_storage.insert(
            commitments[0].as_digest(),
            CommitmentAndOrigins {
                commitment: commitments[0],
                origins: [validator1].iter().cloned().collect(),
            },
        );
        expected_commitments_storage.insert(
            commitments[1].as_digest(),
            CommitmentAndOrigins {
                commitment: commitments[1],
                origins: [validator1].iter().cloned().collect(),
            },
        );
        assert_eq!(expected_commitments_storage, commitments_storage);

        let validator2_private_key = validators_private_keys[1];
        let validator2_pub_key = PublicKey::from(validator2_private_key);
        let validator2 = validator2_pub_key.to_address();

        let aggregated = AggregatedCommitments::aggregate_commitments(
            commitments.to_vec(),
            &signer,
            validator2_pub_key,
            router_address,
        )
        .unwrap();

        Sequencer::receive_commitments(
            aggregated,
            &validators,
            router_address,
            &mut commitments_storage,
        )
        .unwrap();

        expected_commitments_storage
            .get_mut(&commitments[0].as_digest())
            .unwrap()
            .origins
            .insert(validator2);
        expected_commitments_storage
            .get_mut(&commitments[1].as_digest())
            .unwrap()
            .origins
            .insert(validator2);
        assert_eq!(expected_commitments_storage, commitments_storage);
    }

    #[test]
    fn test_set_candidate_commitments() {
        let mut candidate = None;
        let threshold = 2;

        let mut commitments = BTreeMap::new();
        Sequencer::set_candidate_commitments(&commitments, &mut candidate, threshold);
        assert!(candidate.is_none());

        let commitment1 = TestComm([0, 1]);
        let commitment2 = TestComm([2, 3]);
        let commitment3 = TestComm([4, 5]);

        {
            commitments.insert(
                commitment1.as_digest(),
                CommitmentAndOrigins {
                    commitment: commitment1,
                    origins: Default::default(),
                },
            );
            Sequencer::set_candidate_commitments(&commitments, &mut candidate, threshold);
            assert!(candidate.is_none());
        }

        {
            commitments
                .get_mut(&commitment1.as_digest())
                .unwrap()
                .origins
                .insert(Address([1; 20]));
            Sequencer::set_candidate_commitments(&commitments, &mut candidate, threshold);
            assert!(candidate.is_none());
        }

        {
            commitments
                .get_mut(&commitment1.as_digest())
                .unwrap()
                .origins
                .insert(Address([2; 20]));
            Sequencer::set_candidate_commitments(&commitments, &mut candidate, threshold);
            let candidate = candidate.as_ref().unwrap();
            assert_eq!(candidate.digests(), [commitment1.as_digest()].as_slice());
            assert!(candidate.signatures().is_empty());
        }

        {
            commitments.insert(
                commitment2.as_digest(),
                CommitmentAndOrigins {
                    commitment: commitment2,
                    origins: [Address([3; 20]), Address([4; 20])]
                        .iter()
                        .cloned()
                        .collect(),
                },
            );
            Sequencer::set_candidate_commitments(&commitments, &mut candidate, threshold);
            let candidate = candidate.as_ref().unwrap();
            assert_eq!(
                candidate.digests(),
                [commitment1.as_digest(), commitment2.as_digest()].as_slice()
            );
            assert!(candidate.signatures().is_empty());
        }

        {
            commitments.insert(
                commitment3.as_digest(),
                CommitmentAndOrigins {
                    commitment: commitment3,
                    origins: [Address([5; 20])].iter().cloned().collect(),
                },
            );
            Sequencer::set_candidate_commitments(&commitments, &mut candidate, threshold);
            let candidate = candidate.as_ref().unwrap();
            assert_eq!(
                candidate.digests(),
                [commitment1.as_digest(), commitment2.as_digest()].as_slice()
            );
            assert!(candidate.signatures().is_empty());
        }
    }

    #[test]
    #[should_panic(expected = "Guarantied by `Sequencer` implementation to be in the map")]
    fn test_process_multisigned_candidate_empty_map() {
        let candidate =
            MultisignedCommitmentDigests::new([[2; 32]].map(Into::into).into_iter().collect());
        Sequencer::process_multisigned_candidate::<TestComm>(
            &mut Some(candidate),
            &mut Default::default(),
            0,
        );
    }

    #[test]
    fn test_process_multisigned_candidate() {
        let signer = Signer::tmp();

        // Test candidate is None
        assert!(Sequencer::process_multisigned_candidate::<TestComm>(
            &mut None,
            &mut Default::default(),
            0
        )
        .is_none());

        // Test not enough signatures
        let mut candidate = Some(MultisignedCommitmentDigests::new(Default::default()));
        assert!(Sequencer::process_multisigned_candidate(
            &mut candidate,
            &mut CommitmentsMap::<TestComm>::new(),
            2
        )
        .is_none());

        let mut commitments_map = CommitmentsMap::new();
        let validators_private_keys = [PrivateKey([1; 32]), PrivateKey([2; 32])];
        let validators_pub_keys = validators_private_keys.map(|key| signer.add_key(key).unwrap());
        let origins: BTreeSet<_> = validators_pub_keys
            .map(|k| k.to_address())
            .into_iter()
            .collect();

        let commitments = [TestComm([0, 1]), TestComm([2, 3]), TestComm([4, 5])];
        commitments.iter().for_each(|commitment| {
            commitments_map.insert(
                commitment.as_digest(),
                CommitmentAndOrigins {
                    commitment: *commitment,
                    origins: origins.clone(),
                },
            );
        });

        Sequencer::set_candidate_commitments(&commitments_map, &mut candidate, 2);

        let mut candidate = candidate.expect("Must be set");
        let router_address = Address([1; 20]);
        validators_pub_keys.iter().for_each(|pub_key| {
            let commitments_digest = commitments.to_vec().as_digest();
            candidate
                .append_signature_with_check(
                    commitments_digest,
                    agro::sign_commitments_digest(
                        commitments_digest,
                        &signer,
                        *pub_key,
                        router_address,
                    )
                    .unwrap(),
                    router_address,
                    |_| Ok(()),
                )
                .unwrap();
        });

        let mut candidate = Some(candidate);
        assert!(
            Sequencer::process_multisigned_candidate(&mut candidate, &mut commitments_map, 2)
                .is_some(),
        );
        assert!(commitments_map.is_empty());
        assert!(candidate.is_none());
    }
}
