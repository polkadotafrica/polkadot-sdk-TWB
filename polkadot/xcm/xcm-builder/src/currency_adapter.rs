// Copyright (C) Parity Technologies (UK) Ltd.
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

//! Adapters to work with `frame_support::traits::Currency` through XCM.

#![allow(deprecated)]

use super::MintLocation;
use core::{fmt::Debug, marker::PhantomData, result};
use frame_support::traits::{ExistenceRequirement::AllowDeath, Get, WithdrawReasons};
use sp_runtime::traits::CheckedSub;
use xcm::latest::{Asset, Error as XcmError, Location, Result, XcmContext};
use xcm_executor::{
	traits::{ConvertLocation, MatchesFungible, TransactAsset},
	AssetsInHolding,
};

/// Asset transaction errors.
enum Error {
	/// The given asset is not handled. (According to [`XcmError::AssetNotFound`])
	AssetNotHandled,
	/// `Location` to `AccountId` conversion failed.
	AccountIdConversionFailed,
}

impl From<Error> for XcmError {
	fn from(e: Error) -> Self {
		use XcmError::FailedToTransactAsset;
		match e {
			Error::AssetNotHandled => XcmError::AssetNotFound,
			Error::AccountIdConversionFailed => FailedToTransactAsset("AccountIdConversionFailed"),
		}
	}
}

/// Simple adapter to use a currency as asset transactor. This type can be used as `type
/// AssetTransactor` in `xcm::Config`.
///
/// # Example
/// ```
/// use codec::Decode;
/// use frame_support::{parameter_types, PalletId};
/// use sp_runtime::traits::{AccountIdConversion, TrailingZeroInput};
/// use xcm::latest::prelude::*;
/// use staging_xcm_builder::{ParentIsPreset, CurrencyAdapter, IsConcrete};
///
/// /// Our chain's account id.
/// type AccountId = sp_runtime::AccountId32;
///
/// /// Our relay chain's location.
/// parameter_types! {
///     pub RelayChain: Location = Parent.into();
///     pub CheckingAccount: AccountId = PalletId(*b"checking").into_account_truncating();
/// }
///
/// /// Some items that implement `ConvertLocation<AccountId>`. Can be more, but for now we just assume we accept
/// /// messages from the parent (relay chain).
/// pub type LocationConverter = (ParentIsPreset<AccountId>);
///
/// /// Just a dummy implementation of `Currency`. Normally this would be `Balances`.
/// pub type CurrencyImpl = ();
///
/// /// Final currency adapter. This can be used in `xcm::Config` to specify how asset related transactions happen.
/// pub type AssetTransactor = CurrencyAdapter<
///     // Use this `Currency` impl instance:
///     CurrencyImpl,
///     // The matcher: use the currency when the asset is a concrete asset in our relay chain.
///     IsConcrete<RelayChain>,
///     // The local converter: default account of the parent relay chain.
///     LocationConverter,
///     // Our chain's account ID type.
///     AccountId,
///     // The checking account. Can be any deterministic inaccessible account.
///     CheckingAccount,
/// >;
/// ```
#[deprecated = "Use `FungibleAdapter` instead"]
pub struct CurrencyAdapter<Currency, Matcher, AccountIdConverter, AccountId, CheckedAccount>(
	PhantomData<(Currency, Matcher, AccountIdConverter, AccountId, CheckedAccount)>,
);

impl<
		Currency: frame_support::traits::Currency<AccountId>,
		Matcher: MatchesFungible<Currency::Balance>,
		AccountIdConverter: ConvertLocation<AccountId>,
		AccountId: Clone, // can't get away without it since Currency is generic over it.
		CheckedAccount: Get<Option<(AccountId, MintLocation)>>,
	> CurrencyAdapter<Currency, Matcher, AccountIdConverter, AccountId, CheckedAccount>
{
	fn can_accrue_checked(_checked_account: AccountId, _amount: Currency::Balance) -> Result {
		Ok(())
	}
	fn can_reduce_checked(checked_account: AccountId, amount: Currency::Balance) -> Result {
		let new_balance = Currency::free_balance(&checked_account)
			.checked_sub(&amount)
			.ok_or(XcmError::NotWithdrawable)?;
		Currency::ensure_can_withdraw(
			&checked_account,
			amount,
			WithdrawReasons::TRANSFER,
			new_balance,
		)
		.map_err(|error| {
			tracing::debug!(target: "xcm::currency_adapter", ?error, "Failed to ensure can withdraw");
			XcmError::NotWithdrawable
		})
	}
	fn accrue_checked(checked_account: AccountId, amount: Currency::Balance) {
		let _ = Currency::deposit_creating(&checked_account, amount);
		Currency::deactivate(amount);
	}
	fn reduce_checked(checked_account: AccountId, amount: Currency::Balance) {
		let ok =
			Currency::withdraw(&checked_account, amount, WithdrawReasons::TRANSFER, AllowDeath)
				.is_ok();
		if ok {
			Currency::reactivate(amount);
		} else {
			frame_support::defensive!(
				"`can_check_in` must have returned `true` immediately prior; qed"
			);
		}
	}
}

impl<
		Currency: frame_support::traits::Currency<AccountId>,
		Matcher: MatchesFungible<Currency::Balance>,
		AccountIdConverter: ConvertLocation<AccountId>,
		AccountId: Clone + Debug, // can't get away without it since Currency is generic over it.
		CheckedAccount: Get<Option<(AccountId, MintLocation)>>,
	> TransactAsset
	for CurrencyAdapter<Currency, Matcher, AccountIdConverter, AccountId, CheckedAccount>
{
	fn can_check_in(origin: &Location, what: &Asset, _context: &XcmContext) -> Result {
		tracing::trace!(target: "xcm::currency_adapter", ?origin, ?what, "can_check_in origin");
		// Check we handle this asset.
		let amount: Currency::Balance =
			Matcher::matches_fungible(what).ok_or(Error::AssetNotHandled)?;
		match CheckedAccount::get() {
			Some((checked_account, MintLocation::Local)) =>
				Self::can_reduce_checked(checked_account, amount),
			Some((checked_account, MintLocation::NonLocal)) =>
				Self::can_accrue_checked(checked_account, amount),
			None => Ok(()),
		}
	}

	fn check_in(origin: &Location, what: &Asset, _context: &XcmContext) {
		tracing::trace!(target: "xcm::currency_adapter", ?origin, ?what, "check_in origin");
		if let Some(amount) = Matcher::matches_fungible(what) {
			match CheckedAccount::get() {
				Some((checked_account, MintLocation::Local)) =>
					Self::reduce_checked(checked_account, amount),
				Some((checked_account, MintLocation::NonLocal)) =>
					Self::accrue_checked(checked_account, amount),
				None => (),
			}
		}
	}

	fn can_check_out(dest: &Location, what: &Asset, _context: &XcmContext) -> Result {
		tracing::trace!(target: "xcm::currency_adapter", ?dest, ?what, "can_check_out");
		let amount = Matcher::matches_fungible(what).ok_or(Error::AssetNotHandled)?;
		match CheckedAccount::get() {
			Some((checked_account, MintLocation::Local)) =>
				Self::can_accrue_checked(checked_account, amount),
			Some((checked_account, MintLocation::NonLocal)) =>
				Self::can_reduce_checked(checked_account, amount),
			None => Ok(()),
		}
	}

	fn check_out(dest: &Location, what: &Asset, _context: &XcmContext) {
		tracing::trace!(target: "xcm::currency_adapter", ?dest, ?what, "check_out");
		if let Some(amount) = Matcher::matches_fungible(what) {
			match CheckedAccount::get() {
				Some((checked_account, MintLocation::Local)) =>
					Self::accrue_checked(checked_account, amount),
				Some((checked_account, MintLocation::NonLocal)) =>
					Self::reduce_checked(checked_account, amount),
				None => (),
			}
		}
	}

	fn deposit_asset(what: &Asset, who: &Location, _context: Option<&XcmContext>) -> Result {
		tracing::trace!(target: "xcm::currency_adapter", ?what, ?who, "deposit_asset");
		// Check we handle this asset.
		let amount = Matcher::matches_fungible(&what).ok_or(Error::AssetNotHandled)?;
		let who =
			AccountIdConverter::convert_location(who).ok_or(Error::AccountIdConversionFailed)?;
		let _imbalance = Currency::deposit_creating(&who, amount);
		Ok(())
	}

	fn withdraw_asset(
		what: &Asset,
		who: &Location,
		_maybe_context: Option<&XcmContext>,
	) -> result::Result<AssetsInHolding, XcmError> {
		tracing::trace!(target: "xcm::currency_adapter", ?what, ?who, "withdraw_asset");
		// Check we handle this asset.
		let amount = Matcher::matches_fungible(what).ok_or(Error::AssetNotHandled)?;
		let who =
			AccountIdConverter::convert_location(who).ok_or(Error::AccountIdConversionFailed)?;
		let _ = Currency::withdraw(&who, amount, WithdrawReasons::TRANSFER, AllowDeath).map_err(
			|error| {
				tracing::debug!(target: "xcm::currency_adapter", ?error, ?who, ?amount, "Failed to withdraw asset");
				XcmError::FailedToTransactAsset(error.into())
			},
		)?;
		Ok(what.clone().into())
	}

	fn internal_transfer_asset(
		asset: &Asset,
		from: &Location,
		to: &Location,
		_context: &XcmContext,
	) -> result::Result<AssetsInHolding, XcmError> {
		tracing::trace!(target: "xcm::currency_adapter", ?asset, ?from, ?to, "internal_transfer_asset");
		let amount = Matcher::matches_fungible(asset).ok_or(Error::AssetNotHandled)?;
		let from =
			AccountIdConverter::convert_location(from).ok_or(Error::AccountIdConversionFailed)?;
		let to =
			AccountIdConverter::convert_location(to).ok_or(Error::AccountIdConversionFailed)?;
		Currency::transfer(&from, &to, amount, AllowDeath).map_err(|error| {
			tracing::debug!(target: "xcm::currency_adapter", ?error, ?from, ?to, ?amount, "Failed to transfer asset");
			XcmError::FailedToTransactAsset(error.into())
		})?;
		Ok(asset.clone().into())
	}
}
