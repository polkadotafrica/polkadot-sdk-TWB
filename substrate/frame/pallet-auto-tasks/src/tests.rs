#![cfg(test)]

use crate::{mock::*, Error, Event};
use frame_support::{assert_noop, assert_ok};
use sp_runtime::traits::BadOrigin;

#[test]
fn storing_numbers_works() {
    new_test_ext().execute_with(|| {
        // Store a number
        assert_ok!(AutoTasks::store_number(Origin::signed(1), 10, 20));
        
        // Check that it was stored correctly
        assert_eq!(AutoTasks::numbers(10), Some(20));
    });
}

#[test]
fn automated_task_execution_works() {
    new_test_ext().execute_with(|| {
        // Store some numbers
        assert_ok!(AutoTasks::store_number(Origin::signed(1), 10, 20));
        assert_ok!(AutoTasks::store_number(Origin::signed(1), 30, 40));
        
        // Trigger block finalization (this would run the offchain worker)
        AutoTasks::offchain_worker(System::block_number());
        
        // Wait for tasks to execute
        // (In a real chain, this would happen automatically)
        run_to_block(2);
        
        // Check if the totals have been updated
        let (total_keys, total_values) = AutoTasks::total();
        assert_eq!(total_keys, 40); // 10 + 30
        assert_eq!(total_values, 60); // 20 + 40
        
        // The numbers should have been removed from storage
        assert_eq!(AutoTasks::numbers(10), None);
        assert_eq!(AutoTasks::numbers(30), None);
    });
}

#[test]
fn only_signed_users_can_store_numbers() {
    new_test_ext().execute_with(|| {
        // Try to store a number with an unsigned origin
        assert_noop!(
            AutoTasks::store_number(Origin::none(), 10, 20),
            BadOrigin
        );
    });
}