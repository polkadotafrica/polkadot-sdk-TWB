#![cfg(test)]

use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok, weights::Weight};
use sp_runtime::traits::BadOrigin;

#[test]
fn weight_parts_works() {
    // This is a simple test to confirm Weight::from_parts works as expected
    let weight = Weight::from_parts(2_000, 0);
    assert_eq!(weight.ref_time(), 2_000);
    assert_eq!(weight.proof_size(), 0);
}

#[test]
fn storing_numbers_works() {
    new_test_ext().execute_with(|| {
        // Store a number
        assert_ok!(AutoTasks::store_number(RuntimeOrigin::signed(1), 10, 20));
        
        // Check that it was stored correctly
        assert_eq!(AutoTasks::numbers(10), Some(20));
    });
}

#[test]
fn automated_task_execution_works() {
    new_test_ext().execute_with(|| {
        // Store some numbers
        assert_ok!(AutoTasks::store_number(RuntimeOrigin::signed(1), 10, 20));
        assert_ok!(AutoTasks::store_number(RuntimeOrigin::signed(1), 30, 40));
        
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
            AutoTasks::store_number(RuntimeOrigin::none(), 10, 20),
            BadOrigin
        );
    });
}

#[test]
fn add_number_into_total_directly_works() {
    new_test_ext().execute_with(|| {
        // Store a number
        assert_ok!(AutoTasks::store_number(RuntimeOrigin::signed(1), 5, 10));
        
        // Process the number directly
        assert_ok!(AutoTasks::add_number_into_total(5));
        
        // The number should be removed from storage
        assert_eq!(AutoTasks::numbers(5), None);
        
        // And the totals updated
        assert_eq!(AutoTasks::total(), (5, 10));
    });
}

#[test]
fn add_number_fails_when_not_found() {
    new_test_ext().execute_with(|| {
        // Try to process a number that doesn't exist
        assert_noop!(
            AutoTasks::add_number_into_total(99),
            Error::<Test>::NotFound
        );
    });
}