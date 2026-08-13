#![cfg_attr(not(feature = "std"), no_std)]

pub mod conformance;
pub mod types;

pub use pallet::*;

#[cfg(test)]
mod tests {
    mod model;
}

#[frame_support::pallet]
pub mod pallet {
    #[pallet::config]
    pub trait Config: frame_system::Config {}

    #[pallet::pallet]
    pub struct Pallet<T>(_);
}
