mod drones;
mod network_initializer;
mod server;
mod clients;
mod simulation_controller;

use crate::network_initializer::NetworkInit;

fn main() {
    let myNet = NetworkInit::new("input.toml");
}
