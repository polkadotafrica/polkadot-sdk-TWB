#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{dispatch::DispatchResult, pallet_prelude::*};
use frame_system::{offchain::CreateInherent, pallet_prelude::*};

#[cfg(feature = "experimental")]
use frame_system::offchain::SubmitTransaction;

pub use pallet::*;
pub mod weights;
pub use weights::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[cfg(feature = "experimental")]
const LOG_TARGET: &str = "pallet-auto-tasks";

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: CreateInherent<frame_system::Call<Self>> + frame_system::Config {
        type RuntimeTask: frame_support::traits::Task
            + IsType<<Self as frame_system::Config>::RuntimeTask>
            + From<Task<Self>>;
        type WeightInfo: WeightInfo;
    }

    #[pallet::storage]
    pub type Numbers<T: Config> = StorageMap<_, Twox64Concat, u32, u32, OptionQuery>;

    #[pallet::storage]
    pub type Total<T: Config> = StorageValue<_, (u32, u32), ValueQuery>;

    #[pallet::error]
    pub enum Error<T> {
        /// The referenced task was not found.
        NotFound,
    }

    /// Define our task type
    pub enum Task<T: Config> {
        AddNumberIntoTotal { i: u32 },
    }

    #[pallet::tasks_experimental]
    impl<T: Config> Pallet<T> {
        /// Add a number into the totals and remove it from storage.
        #[pallet::task_list(Numbers::<T>::iter_keys())]
        #[pallet::task_condition(|i| Numbers::<T>::contains_key(i))]
        #[pallet::task_weight(T::WeightInfo::add_number_into_total())]
        #[pallet::task_index(0)]
        pub fn add_number_into_total(i: u32) -> DispatchResult {
            // Get the value for this key, remove it from storage
            let v = Numbers::<T>::take(i).ok_or(Error::<T>::NotFound)?;
            
            // Add both the key and value to our running totals
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
        fn offchain_worker(_block_number: BlockNumberFor<T>) {
            if let Some(key) = Numbers::<T>::iter_keys().next() {
                // Create a valid task
                let task = Task::<T>::AddNumberIntoTotal { i: key };
                let runtime_task = <T as Config>::RuntimeTask::from(task);
                let call = frame_system::Call::<T>::do_task { task: runtime_task.into() };
                
                // Submit the task as an inherent transaction
                let xt = <T as CreateInherent<frame_system::Call<T>>>::create_inherent(call.into());
                let res = SubmitTransaction::<T, frame_system::Call<T>>::submit_transaction(xt);
                
                match res {
                    Ok(_) => log::info!(target: LOG_TARGET, "Submitted the task."),
                    Err(e) => log::error!(target: LOG_TARGET, "Error submitting task: {:?}", e),
                }
            }
        }

        #[cfg(not(feature = "experimental"))]
        fn offchain_worker(_block_number: BlockNumberFor<T>) {}
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Store a new number to be processed by the automated task.
        #[pallet::weight(T::WeightInfo::store_number())]
        pub fn store_number(origin: OriginFor<T>, key: u32, value: u32) -> DispatchResult {
            ensure_signed(origin)?;
            Numbers::<T>::insert(key, value);
            Ok(())
        }

        /// View the current accumulated totals.
        #[pallet::weight(T::WeightInfo::get_totals())]
        pub fn get_totals(origin: OriginFor<T>) -> DispatchResult {
            ensure_signed(origin)?;
            let (total_keys, total_values) = Total::<T>::get();
            log::info!(
                target: "get_totals",
                "Current totals - Keys: {}, Values: {}",
                total_keys,
                total_values
            );
            Ok(())
        }
    }

    /// Genesis configuration for our pallet
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