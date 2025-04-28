#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        HelloWorldStored { comment: u32, who: T::AccountId },
    }

    #[pallet::storage]
    pub type HelloWorldStorage<T: Config> = StorageValue<_, u32>;

    #[pallet::error]
    pub enum Error<T> {
        NoHelloWorld,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::default())]
        pub fn hello_world(origin: OriginFor<T>, comment: u32) -> DispatchResult {
            let who = ensure_signed(origin)?;

            HelloWorldStorage::<T>::put(comment);

            Self::deposit_event(Event::<T>::HelloWorldStored { who, comment });

            Ok(())
        }
    }
}
