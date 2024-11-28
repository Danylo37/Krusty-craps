mod drones;
mod network_initializer;
mod servers;
mod clients;

use crate::network_initializer::NetworkInit;

fn main() {
    let myNet = NetworkInit::new("input.toml");
}
