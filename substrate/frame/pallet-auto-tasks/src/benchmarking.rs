//! Benchmarking for pallet-auto-tasks

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;

benchmarks! {
	add_number_into_total {
		let i in 1..100;
		let v = 1000;
		Numbers::<T>::insert(i, v);
	}: {
		Pallet::<T>::add_number_into_total(i).unwrap();
	}
	verify {
		assert!(Numbers::<T>::get(i).is_none());
		assert_eq!(Total::<T>::get(), (i, v));
	}

	store_number {
		let caller: T::AccountId = whitelisted_caller();
		let i = 42u32;
		let v = 100u32;
	}: _(RawOrigin::Signed(caller), i, v)
	verify {
		assert_eq!(Numbers::<T>::get(i), Some(v));
	}

	get_totals {
		let caller: T::AccountId = whitelisted_caller();
		Total::<T>::put((123u32, 456u32));
	}: _(RawOrigin::Signed(caller))
	verify {
		assert_eq!(Total::<T>::get(), (123u32, 456u32));
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
