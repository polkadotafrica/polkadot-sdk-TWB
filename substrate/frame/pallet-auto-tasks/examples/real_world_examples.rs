// This file contains real-world examples of automated tasks implementations
// that can be incorporated into the main pallet

// Example 1: Automatic Message Cleanup
#[allow(dead_code)]
mod message_cleanup_example {
    use frame_support::{dispatch::DispatchResult, pallet_prelude::*};
    use frame_system::pallet_prelude::*;

    type MessageId = u32;
    type MessageContent = Vec<u8,>;

    // Define storage for messages with expiry information
    #[pallet::storage]
    pub type Messages<T: Config,> = StorageMap<
        _,
        Twox64Concat,
        MessageId,
        (MessageContent, BlockNumberFor<T,>,), // Content and expiry block
        OptionQuery,
    >;

    // Define events
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config,> {
        MessageExpired(MessageId,),
    }

    // Define the cleanup task
    #[pallet::tasks_experimental]
    impl<T: Config,> Pallet<T,> {
        // Task to clean up expired messages
        #[pallet::task_list(Messages::<T>::iter_keys())]
        #[pallet::task_condition(|msg_id| {
			if let Some((_, expiry)) = Messages::<T>::get(msg_id) {
				// Check if message has expired
				frame_system::Pallet::<T>::block_number() >= expiry
			} else {
				false
			}
		})]
        #[pallet::task_weight(T::WeightInfo::clean_expired_message())]
        #[pallet::task_index(1)] // Note the unique index
        pub fn clean_expired_message(msg_id: MessageId,) -> DispatchResult {
            // Simply remove the expired message
            Messages::<T,>::remove(msg_id,);

            // Emit an event
            Self::deposit_event(Event::MessageExpired(msg_id,),);

            Ok((),)
        }
    }
}

// Example 2: Data Aggregation Service
#[allow(dead_code)]
mod data_aggregation_example {
    use frame_support::{dispatch::DispatchResult, pallet_prelude::*};
    use frame_system::pallet_prelude::*;

    type Balance = u128;

    // Storage for transaction values
    #[pallet::storage]
    pub type TransactionValues<T: Config,> =
        StorageMap<_, Twox64Concat, BlockNumberFor<T,>, Vec<Balance,>, ValueQuery,>;

    // Storage for processed blocks by day
    #[pallet::storage]
    pub type DailyProcessedBlocks<T: Config,> = StorageMap<
        _,
        Twox64Concat,
        u32,                      // Day number
        Vec<BlockNumberFor<T,>,>, // Processed blocks
        ValueQuery,
    >;

    // Storage for aggregated statistics
    #[pallet::storage]
    pub type DailyAverages<T: Config,> = StorageMap<
        _,
        Twox64Concat,
        u32,     // Day number
        Balance, // Average transaction value
        ValueQuery,
    >;

    // Helper functions for the pallet
    impl<T: Config,> Pallet<T,> {
        // Calculate current day number based on block number
        pub fn calculate_day_number() -> u32 {
            let current_block = frame_system::Pallet::<T,>::block_number();
            let blocks_per_day = 7200u32.into(); // Assuming 7200 blocks per day
            let day = current_block / blocks_per_day;

            // Convert from BlockNumberFor<T> to u32
            let day_as_u32: u32 = day.try_into().unwrap_or_default();
            day_as_u32
        }

        // Calculate day from a specific block number
        pub fn calculate_day_from_block(block_num: BlockNumberFor<T,>,) -> u32 {
            let blocks_per_day = 7200u32.into(); // Assuming 7200 blocks per day
            let day = block_num / blocks_per_day;

            // Convert from BlockNumberFor<T> to u32
            let day_as_u32: u32 = day.try_into().unwrap_or_default();
            day_as_u32
        }
    }

    // Define the aggregation task
    #[pallet::tasks_experimental]
    impl<T: Config,> Pallet<T,> {
        // Task to aggregate daily transaction statistics
        #[pallet::task_list({
			// Get a list of blocks from the current day that need processing
			let current_day = Self::calculate_day_number();
			let processed = DailyProcessedBlocks::<T>::get(current_day);
			TransactionValues::<T>::iter_keys()
				.filter(|block_num| !processed.contains(block_num))
				.collect::<Vec<_>>()
		})]
        #[pallet::task_condition(|block_num| {
			// Only process blocks from completed days
			let block_day = Self::calculate_day_from_block(*block_num);
			let current_day = Self::calculate_day_number();
			block_day < current_day
		})]
        #[pallet::task_weight(T::WeightInfo::aggregate_daily_stats())]
        #[pallet::task_index(2)]
        pub fn aggregate_daily_stats(block_num: BlockNumberFor<T,>,) -> DispatchResult {
            // Get transaction values for this block
            let values = TransactionValues::<T,>::get(block_num,);
            if values.is_empty() {
                return Ok((),);
            }

            // Calculate average
            let sum: Balance = values.iter().sum();
            let avg = sum / (values.len() as u32).into();

            // Get the day this block belongs to
            let day = Self::calculate_day_from_block(block_num,);

            // Update daily average (using weighted average for accuracy)
            DailyAverages::<T,>::mutate(day, |current_avg| {
                let processed_blocks = DailyProcessedBlocks::<T,>::get(day,);
                let processed_count = processed_blocks.len() as u32;

                if processed_count == 0 {
                    *current_avg = avg;
                } else {
                    *current_avg = (*current_avg * processed_count.into() + avg)
                        / (processed_count + 1).into();
                }
            },);

            // Mark this block as processed
            DailyProcessedBlocks::<T,>::mutate(day, |blocks| {
                blocks.push(block_num,);
            },);

            Ok((),)
        }
    }
}
