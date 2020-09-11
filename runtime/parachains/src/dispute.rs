// Copyright 2020 Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! The dispute module is responsible for resolving disputes that appear after block inclusion.
//!
//! It is responsible for collecting votes from validators after an initial local dispute as well
//! as crafting transactions using the provisioner for slashing the validators on the wrong side.

use sp_std::prelude::*;
use primitives::v1::{
	ValidatorId, CandidateCommitments, CandidateDescriptor, ValidatorIndex, Id as ParaId,
	AvailabilityBitfield as AvailabilityBitfield, SignedAvailabilityBitfields, SigningContext,
	BackedCandidate, CoreIndex, GroupIndex, CommittedCandidateReceipt,
	CandidateReceipt, HeadData,
};
use frame_support::{
	decl_storage, decl_module, decl_error, decl_event, ensure, debug,
	dispatch::DispatchResult, IterableStorageMap, weights::Weight, traits::Get,
};
use codec::{Encode, Decode};
use bitvec::{order::Lsb0 as BitOrderLsb0, vec::BitVec};
use sp_staking::SessionIndex;
use sp_runtime::{DispatchError, traits::{One, Saturating}};

use crate::{configuration, paras, scheduler::CoreAssignment};

#[derive(Encode, Decode)]
#[cfg_attr(test, derive(Debug))]
pub struct AvailabilityBitfieldRecord<N> {
	bitfield: AvailabilityBitfield, // one bit per core.
	submitted_at: N, // for accounting, as meaning of bits may change over time.
}

/// A backed candidate pending availability.
// TODO: split this type and change this to hold a plain `CandidateReceipt`.
// https://github.com/paritytech/polkadot/issues/1357
#[derive(Encode, Decode, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub struct CandidatePendingAvailability<H, N> {
	/// The availability core this is assigned to.
	core: CoreIndex,
	/// The candidate descriptor.
	descriptor: CandidateDescriptor<H>,
	/// The received availability votes. One bit per validator.
	availability_votes: BitVec<BitOrderLsb0, u8>,
	/// The block number of the relay-parent of the receipt.
	relay_parent_number: N,
	/// The block number of the relay-chain block this was backed in.
	backed_in_number: N,
}

pub trait Trait:
	frame_system::Trait + paras::Trait + configuration::Trait
{
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

decl_storage! {
	trait Store for Module<T: Trait> as Dispute {
		/// The vote of the selected validators.
		ValidatorVotes: map hasher(twox_64_concat) ValidatorIndex
			=> Option<bool>;

		/// The commitments of candidates pending availability, by ParaId.
		PendingAvailabilityCommitments: map hasher(twox_64_concat) ParaId
			=> Option<CandidateCommitments>;

		/// The current validators, by their parachain session keys.
		Validators get(fn validators) config(validators): Vec<ValidatorId>;

		/// The current session index.
		CurrentSessionIndex get(fn session_index): SessionIndex;
	}
}

// Errors inform users that something went wrong.
decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Error Y.
		Y,
		/// Error X.
		X,
	}
}

// only for 3rd party apps, not for internal usage 
decl_event! {
	pub enum Event<T> where <T as frame_system::Trait>::Hash, <T as frame_system::Trait>::BlockNumber {
        /// An indication of one validator that something is off. []
        DisputeIndicated(CandidateReceipt<Hash>, SessionIndex, BlockNumber), // TODO what over checks must there be included? secondary
		/// A dispute resolved with an outcome. []
		DisputeResolved(CandidateReceipt<Hash>, SessionIndex, BlockNumber),
		/// A candidate timed out. []
		DisputeTimedOut(CandidateReceipt<Hash>, HeadData),
	}
}

decl_module! {
	/// The parachain-candidate dispute module.
	pub struct Module<T: Trait>
		for enum Call where origin: <T as frame_system::Trait>::Origin
	{
		fn deposit_event() = default;
	}
}

impl<T: Trait> Module<T> {

	/// Block initialization logic, called by initializer.
	pub(crate) fn initializer_initialize(_now: T::BlockNumber) -> Weight { 0 }

	/// Block finalization logic, called by initializer.
	pub(crate) fn initializer_finalize() { }

	/// Handle an incoming session change.
	pub(crate) fn initializer_on_new_session(
		notification: &crate::initializer::SessionChangeNotification<T::BlockNumber>
	) {
		// unlike most drain methods, drained elements are not cleared on `Drop` of the iterator
		// and require consumption.
		for _ in <ValidatorVotes>::drain() { }
    }
    

    fn validators_pro() -> Vec<ValidatorId> {
        vec![] // TODO
    }

    fn validators_cons() -> Vec<ValidatorId> {
        vec![] // TODO
    }

    /// The set of validators which originally validated that block.
    fn original_validating_valdiators(session: SessionIndex) -> Vec<ValidatorId> {
        unimplemented!("");
    }

    /// Check all of the known votes in storage for that block.
    /// Returns `true`
    fn count_pro_and_cons_votes(block: <T as frame_system::Trait>::Hash) -> DisputeVotes {
        // TODO which votes to we count here?
        // approval?
        // backing?
        // both?
        DisputeVotes::default() // TODO
    }


    /// Transplant a vote onto all other forks.
    fn transplant_to(resolution: Resolution, active_heads: Vec<<T as frame_system::Trait>::Hash>) {

    }

    /// Extend the set of blocks to never sync again.
    fn extend_blacklist(burnt: &[<T as frame_system::Trait>::Hash]) {
        unimplemented!("Use that other module impl")
    }

    // block the block number in question
    //
    pub(crate) fn process_concluded(
        block_number: <T as frame_system::Trait>::BlockNumber,
        block_hash: <T as frame_system::Trait>::Hash,
        session: SessionIndex) -> Result<(), DispatchError>
    {
        // TODO ensure!(..), bounds unclear

        // number of _all_ validators
        let all_validators = 10u32; // TODO disamibiguate
        let DisputeVotes { pro, cons } = Self::count_pro_and_cons_votes(block_hash);
        let thresh = resolution_threshold(all_validators.len()) as u32;
        let (pro, cons) = (pro >= thresh, cons >= thresh);

        if !(pro ^ cons) {
            return Err(Error::X)
        } else if pro && cons {
            unreachable!("The number of validators was correctly assessed. qed");
        } else if !pro && !cons {
            // nothing todo just yet
            return Ok(())
        }

        let resolution = if cons {
            Self::extend_blacklist(&[block_hash]);
            // slash the other party
            Resolution {
                hash: block_hash,
                to_punish: Self::validators_pro(),
                was_truely_wrong: true,
            }
        } else if pro {
            // slash the other party
            Resolution {
                hash: block_hash,
                to_punish: Self::validators_cons(),
                was_truely_wrong: false,
            }
        } else {
            return Err(Error::Y)
        };


        let active_heads = vec![]; // TODO extract from the runtime, is this correct? 

        // 
        Self::transplant_to(resolution, active_heads);

        // the original validators screwed up too, so slashing is in order
        let original_offenders = Self::original_validating_valdiators(session);

        Ok(())
    }
}

#[derive(Encode, Decode)]
struct Resolution {
    hash: Hash, // hash of the storage root / state root this dispute was about
    was_truely_wrong: bool, // if the originally tagged as bad, was actually bad
    to_punish: Vec<ValidatorId>, // the validator party to slash
}

#[derive(Encode, Decode, Default)]
pub(crate) struct DisputeVotes {
    pub(crate) pro: u32,
    pub(crate) cons: u32,
}

/// Calculate the majority requred to sway in one way or another
const fn resolution_threshold(n_validators: usize) -> usize {
	let mut threshold = (n_validators * 2) / 3;
	threshold += (n_validators * 2) % 3;
	threshold
}

#[cfg(test)]
mod tests {
    use super::*;
    

    #[test]
    fn f() {
       assert!(true);
    }
}
