#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    pallet_prelude::*,
    traits::{IsType, Task},
    dispatch::DispatchResult,
};
#[cfg(feature = "experimental")]
use frame_system::offchain::SubmitTransaction;
use frame_system::{
    pallet_prelude::*,
    offchain::CreateInherent,
};

pub use pallet::*;
pub mod weights;
pub use weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + CreateInherent<Call<Self>> {
        type RuntimeTask: Task 
            + IsType<<Self as frame_system::Config>::RuntimeTask>
            + From<Task<Self>>;
        type WeightInfo: crate::weights::WeightInfo;
    }

    #[pallet::storage]
    pub type Numbers<T: Config> = StorageMap<_, Twox64Concat, u32, u32, OptionQuery>;

    #[pallet::storage]
    pub type Total<T: Config> = StorageValue<_, (u32, u32), ValueQuery>;

    #[pallet::error]
    pub enum Error<T> {
        NotFound,
    }

    pub enum Task<T: Config> {
        AddNumberIntoTotal { i: u32 },
    }

    #[pallet::tasks_experimental]
    impl<T: Config> Pallet<T> {
        #[pallet::task_list(Numbers::<T>::iter_keys())]
        #[pallet::task_condition(|i| Numbers::<T>::contains_key(i))]
        #[pallet::task_weight(T::WeightInfo::add_number_into_total())]
        #[pallet::task_index(0)]
        pub fn add_number_into_total(i: u32) -> DispatchResult {
            let v = Numbers::<T>::take(i).ok_or(Error::<T>::NotFound)?;
            
            Total::<T>::mutate(|(total_keys, total_values)| {
                *total_keys += i;
                *total_values += v;
            });

            Ok(())
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        #[cfg(feature = "experimental")]
        fn offchain_worker(block_number: BlockNumberFor<T>) {
            if let Some(key) = Numbers::<T>::iter_keys().next() {
                let task = Task::<T>::AddNumberIntoTotal { i: key };
                let runtime_task = <T as Config>::RuntimeTask::from(task);
                let call = Call::<T>::do_task { task: runtime_task.into() };

                let xt = <T as CreateInherent<Call<T>>>::create_inherent(call.into());
                let _ = SubmitTransaction::<T, Call<T>>::submit_transaction(xt);
            }
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(T::WeightInfo::store_number())]
        pub fn store_number(origin: OriginFor<T>, key: u32, value: u32) -> DispatchResult {
            ensure_signed(origin)?;
            Numbers::<T>::insert(key, value);
            Ok(())
        }

        #[pallet::weight(T::WeightInfo::get_totals())]
        pub fn get_totals(origin: OriginFor<T>) -> DispatchResult {
            ensure_signed(origin)?;
            let (keys, values) = Total::<T>::get();
            frame_support::log::info!("Totals - Keys: {}, Values: {}", keys, values);
            Ok(())
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig {
        pub initial_numbers: Vec<(u32, u32)>,
    }

    #[cfg(feature = "std")]
    impl Default for GenesisConfig {
        fn default() -> Self {
            Self { initial_numbers: Vec::new() }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {
            for (k, v) in &self.initial_numbers {
                Numbers::<T>::insert(k, v);
            }
        }
    }
}