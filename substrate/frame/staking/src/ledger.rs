// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A Ledger implementation for stakers.
//!
//! A [`StakingLedger`] encapsulates all the state and logic related to the stake of bonded
//! stakers, namely, it handles the following storage items:
//! * [`Bonded`]: mutates and reads the state of the controller <> stash bond map (to be deprecated
//! soon);
//! * [`Ledger`]: mutates and reads the state of all the stakers. The [`Ledger`] storage item stores
//!   instances of [`StakingLedger`] keyed by the staker's controller account and should be mutated
//!   and read through the [`StakingLedger`] API;
//! * [`Payee`]: mutates and reads the reward destination preferences for a bonded stash.
//! * Staking locks: mutates the locks for staking.
//!
//! NOTE: All the storage operations related to the staking ledger (both reads and writes) *MUST* be
//! performed through the methods exposed by the [`StakingLedger`] implementation in order to ensure
//! state consistency.

use frame_support::{defensive, ensure, traits::Defensive};
use sp_runtime::DispatchResult;
use sp_staking::{StakingAccount, StakingInterface};

use crate::{
	asset, BalanceOf, Bonded, Config, Error, Ledger, Pallet, Payee, RewardDestination,
	StakingLedger, VirtualStakers,
};

#[cfg(any(feature = "runtime-benchmarks", test))]
use sp_runtime::traits::Zero;

impl<T: Config> StakingLedger<T> {
	#[cfg(any(feature = "runtime-benchmarks", test))]
	pub fn default_from(stash: T::AccountId) -> Self {
		Self {
			stash: stash.clone(),
			total: Zero::zero(),
			active: Zero::zero(),
			unlocking: Default::default(),
			legacy_claimed_rewards: Default::default(),
			controller: Some(stash),
		}
	}

	/// Returns a new instance of a staking ledger.
	///
	/// The [`Ledger`] storage is not mutated. In order to store, `StakingLedger::update` must be
	/// called on the returned staking ledger.
	///
	/// Note: as the controller accounts are being deprecated, the stash account is the same as the
	/// controller account.
	pub fn new(stash: T::AccountId, stake: BalanceOf<T>) -> Self {
		Self {
			stash: stash.clone(),
			active: stake,
			total: stake,
			unlocking: Default::default(),
			legacy_claimed_rewards: Default::default(),
			// controllers are deprecated and mapped 1-1 to stashes.
			controller: Some(stash),
		}
	}

	/// Returns the paired account, if any.
	///
	/// A "pair" refers to the tuple (stash, controller). If the input is a
	/// [`StakingAccount::Stash`] variant, its pair account will be of type
	/// [`StakingAccount::Controller`] and vice-versa.
	///
	/// This method is meant to abstract from the runtime development the difference between stash
	/// and controller. This will be deprecated once the controller is fully deprecated as well.
	pub(crate) fn paired_account(account: StakingAccount<T::AccountId>) -> Option<T::AccountId> {
		match account {
			StakingAccount::Stash(stash) => <Bonded<T>>::get(stash),
			StakingAccount::Controller(controller) =>
				<Ledger<T>>::get(&controller).map(|ledger| ledger.stash),
		}
	}

	/// Returns whether a given account is bonded.
	pub(crate) fn is_bonded(account: StakingAccount<T::AccountId>) -> bool {
		match account {
			StakingAccount::Stash(stash) => <Bonded<T>>::contains_key(stash),
			StakingAccount::Controller(controller) => <Ledger<T>>::contains_key(controller),
		}
	}

	/// Returns a staking ledger, if it is bonded and it exists in storage.
	///
	/// This getter can be called with either a controller or stash account, provided that the
	/// account is properly wrapped in the respective [`StakingAccount`] variant. This is meant to
	/// abstract the concept of controller/stash accounts from the caller.
	///
	/// Returns [`Error::BadState`] when a bond is in "bad state". A bond is in a bad state when a
	/// stash has a controller which is bonding a ledger associated with another stash.
	pub(crate) fn get(account: StakingAccount<T::AccountId>) -> Result<StakingLedger<T>, Error<T>> {
		let (stash, controller) = match account.clone() {
			StakingAccount::Stash(stash) =>
				(stash.clone(), <Bonded<T>>::get(&stash).ok_or(Error::<T>::NotStash)?),
			StakingAccount::Controller(controller) => (
				Ledger::<T>::get(&controller)
					.map(|l| l.stash)
					.ok_or(Error::<T>::NotController)?,
				controller,
			),
		};

		let ledger = <Ledger<T>>::get(&controller)
			.map(|mut ledger| {
				ledger.controller = Some(controller.clone());
				ledger
			})
			.ok_or(Error::<T>::NotController)?;

		// if ledger bond is in a bad state, return error to prevent applying operations that may
		// further spoil the ledger's state. A bond is in bad state when the bonded controller is
		// associated with a different ledger (i.e. a ledger with a different stash).
		//
		// See <https://github.com/paritytech/polkadot-sdk/issues/3245> for more details.
		ensure!(
			Bonded::<T>::get(&stash) == Some(controller) && ledger.stash == stash,
			Error::<T>::BadState
		);

		Ok(ledger)
	}

	/// Returns the reward destination of a staking ledger, stored in [`Payee`].
	///
	/// Note: if the stash is not bonded and/or does not have an entry in [`Payee`], it returns the
	/// default reward destination.
	pub(crate) fn reward_destination(
		account: StakingAccount<T::AccountId>,
	) -> Option<RewardDestination<T::AccountId>> {
		let stash = match account {
			StakingAccount::Stash(stash) => Some(stash),
			StakingAccount::Controller(controller) =>
				Self::paired_account(StakingAccount::Controller(controller)),
		};

		if let Some(stash) = stash {
			<Payee<T>>::get(stash)
		} else {
			defensive!("fetched reward destination from unbonded stash {}", stash);
			None
		}
	}

	/// Returns the controller account of a staking ledger.
	///
	/// Note: it will fallback into querying the [`Bonded`] storage with the ledger stash if the
	/// controller is not set in `self`, which most likely means that self was fetched directly from
	/// [`Ledger`] instead of through the methods exposed in [`StakingLedger`]. If the ledger does
	/// not exist in storage, it returns `None`.
	pub fn controller(&self) -> Option<T::AccountId> {
		self.controller.clone().or_else(|| {
			defensive!("fetched a controller on a ledger instance without it.");
			Self::paired_account(StakingAccount::Stash(self.stash.clone()))
		})
	}

	/// Inserts/updates a staking ledger account.
	///
	/// Bonds the ledger if it is not bonded yet, signalling that this is a new ledger. The staking
	/// locks of the stash account are updated accordingly.
	///
	/// Note: To ensure lock consistency, all the [`Ledger`] storage updates should be made through
	/// this helper function.
	pub(crate) fn update(self) -> Result<(), Error<T>> {
		if !<Bonded<T>>::contains_key(&self.stash) {
			return Err(Error::<T>::NotStash)
		}

		// We skip locking virtual stakers.
		if !Pallet::<T>::is_virtual_staker(&self.stash) {
			// for direct stakers, update lock on stash based on ledger.
			asset::update_stake::<T>(&self.stash, self.total)
				.map_err(|_| Error::<T>::NotEnoughFunds)?;
		}

		Ledger::<T>::insert(
			&self.controller().ok_or_else(|| {
				defensive!("update called on a ledger that is not bonded.");
				Error::<T>::NotController
			})?,
			&self,
		);

		Ok(())
	}

	/// Bonds a ledger.
	///
	/// It sets the reward preferences for the bonded stash.
	pub(crate) fn bond(self, payee: RewardDestination<T::AccountId>) -> Result<(), Error<T>> {
		if <Bonded<T>>::contains_key(&self.stash) {
			return Err(Error::<T>::AlreadyBonded)
		}

		<Payee<T>>::insert(&self.stash, payee);
		<Bonded<T>>::insert(&self.stash, &self.stash);
		self.update()
	}

	/// Sets the ledger Payee.
	pub(crate) fn set_payee(self, payee: RewardDestination<T::AccountId>) -> Result<(), Error<T>> {
		if !<Bonded<T>>::contains_key(&self.stash) {
			return Err(Error::<T>::NotStash)
		}

		<Payee<T>>::insert(&self.stash, payee);
		Ok(())
	}

	/// Sets the ledger controller to its stash.
	pub(crate) fn set_controller_to_stash(self) -> Result<(), Error<T>> {
		let controller = self.controller.as_ref()
            .defensive_proof("Ledger's controller field didn't exist. The controller should have been fetched using StakingLedger.")
            .ok_or(Error::<T>::NotController)?;

		ensure!(self.stash != *controller, Error::<T>::AlreadyPaired);

		// check if the ledger's stash is a controller of another ledger.
		if let Some(bonded_ledger) = Ledger::<T>::get(&self.stash) {
			// there is a ledger bonded by the stash. In this case, the stash of the bonded ledger
			// should be the same as the ledger's stash. Otherwise fail to prevent data
			// inconsistencies. See <https://github.com/paritytech/polkadot-sdk/pull/3639> for more
			// details.
			ensure!(bonded_ledger.stash == self.stash, Error::<T>::BadState);
		}

		<Ledger<T>>::remove(&controller);
		<Ledger<T>>::insert(&self.stash, &self);
		<Bonded<T>>::insert(&self.stash, &self.stash);

		Ok(())
	}

	/// Clears all data related to a staking ledger and its bond in both [`Ledger`] and [`Bonded`]
	/// storage items and updates the stash staking lock.
	pub(crate) fn kill(stash: &T::AccountId) -> DispatchResult {
		let controller = <Bonded<T>>::get(stash).ok_or(Error::<T>::NotStash)?;

		<Ledger<T>>::get(&controller).ok_or(Error::<T>::NotController).map(|ledger| {
			Ledger::<T>::remove(controller);
			<Bonded<T>>::remove(&stash);
			<Payee<T>>::remove(&stash);

			// kill virtual staker if it exists.
			if <VirtualStakers<T>>::take(&ledger.stash).is_none() {
				// if not virtual staker, clear locks.
				asset::kill_stake::<T>(&ledger.stash)?;
			}

			Ok(())
		})?
	}
}

#[cfg(test)]
use {
	crate::UnlockChunk,
	codec::{Decode, Encode, MaxEncodedLen},
	scale_info::TypeInfo,
};

// This structs makes it easy to write tests to compare staking ledgers fetched from storage. This
// is required because the controller field is not stored in storage and it is private.
#[cfg(test)]
#[derive(frame_support::DebugNoBound, Clone, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct StakingLedgerInspect<T: Config> {
	pub stash: T::AccountId,
	#[codec(compact)]
	pub total: BalanceOf<T>,
	#[codec(compact)]
	pub active: BalanceOf<T>,
	pub unlocking: frame_support::BoundedVec<UnlockChunk<BalanceOf<T>>, T::MaxUnlockingChunks>,
	pub legacy_claimed_rewards: frame_support::BoundedVec<sp_staking::EraIndex, T::HistoryDepth>,
}

#[cfg(test)]
impl<T: Config> PartialEq<StakingLedgerInspect<T>> for StakingLedger<T> {
	fn eq(&self, other: &StakingLedgerInspect<T>) -> bool {
		self.stash == other.stash &&
			self.total == other.total &&
			self.active == other.active &&
			self.unlocking == other.unlocking &&
			self.legacy_claimed_rewards == other.legacy_claimed_rewards
	}
}

#[cfg(test)]
impl<T: Config> codec::EncodeLike<StakingLedger<T>> for StakingLedgerInspect<T> {}
