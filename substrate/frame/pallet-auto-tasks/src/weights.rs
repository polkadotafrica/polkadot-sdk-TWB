use frame_support::weights::Weight;

pub trait WeightInfo {
    fn add_number_into_total() -> Weight;
    fn store_number() -> Weight;
    fn get_totals() -> Weight;
}

// Default implementation (for testing)
impl WeightInfo for () {
    fn add_number_into_total() -> Weight {
        Weight::from_parts(10_000, 0,)
    }
    fn store_number() -> Weight {
        Weight::from_parts(5_000, 0,)
    }
    fn get_totals() -> Weight {
        Weight::from_parts(2_000, 0,)
    }
}

// SubstrateWeight implementation
pub struct SubstrateWeight<T,>(core::marker::PhantomData<T,>,);
impl<T,> WeightInfo for SubstrateWeight<T,> {
    fn add_number_into_total() -> Weight {
        Weight::from_parts(10_000, 0,)
    }
    fn store_number() -> Weight {
        Weight::from_parts(5_000, 0,)
    }
    fn get_totals() -> Weight {
        Weight::from_parts(2_000, 0,)
    }
}
