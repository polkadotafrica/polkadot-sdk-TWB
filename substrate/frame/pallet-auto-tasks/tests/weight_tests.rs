use frame_support::weights::Weight;
use pallet_auto_tasks::weights::WeightInfo;

#[test]
fn test_weight_implementations() {
	assert_eq!(<() as WeightInfo>::add_number_into_total(), Weight::from_parts(10_000, 0));
	assert_eq!(<() as WeightInfo>::store_number(), Weight::from_parts(5_000, 0));
	assert_eq!(<() as WeightInfo>::get_totals(), Weight::from_parts(2_000, 0));
}
