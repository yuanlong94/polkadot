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
		ValidatorVote: map hasher(twox_64_concat) ValidatorIndex
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

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// TODO, seriously
		SoWrong,
	}
}

decl_event! {
	pub enum Event<T> where <T as frame_system::Trait>::Hash {
        /// An indication of one validator that something is off.
        DisputeIndicated(CandidateReceipt<Hash>, SessionIndex, T::BlockNumber), // TODO what over checks must there be included? secondary
		/// A dispute resolved with an outcome.
		DisputeResolved(CandidateReceipt<Hash>, Hash, HeadData),
		/// A candidate timed out.
		DisputeTimedOut(CandidateReceipt<Hash>, HeadData),
	}
}

decl_module! {
	/// The parachain-candidate dispute module.
	pub struct Module<T: Trait>
		for enum Call where origin: <T as frame_system::Trait>::Origin
	{
		type Error = Error<T>;

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

}

/// Calculate the majority requred to sway in one way or another
const fn sway_threshold(n_validators: usize) -> usize {
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
