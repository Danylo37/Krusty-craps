#![cfg(test)]
use crate::drone::RustyDrone;
use wg_2024::tests::{
    generic_chain_fragment_ack, generic_chain_fragment_drop, generic_fragment_drop,
    generic_fragment_forward,
};

#[test]
pub fn test_generic_fragment_forward() {
    generic_fragment_forward::<RustyDrone>();
}

#[test]
pub fn test_generic_fragment_drop() {
    generic_fragment_drop::<RustyDrone>();
}

#[test]
pub fn test_generic_chain_fragment_drop() {
    generic_chain_fragment_drop::<RustyDrone>();
}

#[test]
pub fn test_generic_chain_fragment_ack() {
    generic_chain_fragment_ack::<RustyDrone>();
}
