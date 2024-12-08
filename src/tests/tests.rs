use crate::tests::{
    test_fragments::*,
    test_flooding::*,
};

use crate::drones::KrustyCrapDrone;

#[cfg(test)]

#[test]
fn test_generic_fragment_forward() {
    generic_fragment_forward::<KrustyCrapDrone>();
}

#[test]
fn test_generic_fragment_drop() {
    generic_fragment_drop::<KrustyCrapDrone>();
}

#[test]
fn test_generic_chain_fragment_drop() {
    generic_chain_fragment_drop::<KrustyCrapDrone>();
}

#[test]
fn test_generic_chain_fragment_ack() {
    generic_chain_fragment_ack::<KrustyCrapDrone>();
}

#[test]
fn test_generic_flood_request_forward() {
    generic_flood_request_forward::<KrustyCrapDrone>();
}
