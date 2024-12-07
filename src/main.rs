mod drones;

//Mod components
mod network_initializer;
mod server;
mod clients;
mod simulation_controller;
mod general_use;

//Test
mod tests;
mod ui;

//Usages
use crate::network_initializer::NetworkInit;

fn main() {
    let myNet = NetworkInit::new("input.toml");
}
