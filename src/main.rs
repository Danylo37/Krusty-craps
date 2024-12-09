//Mod components
mod network_initializer;
mod server;
mod clients;
mod simulation_controller;
mod general_use;
mod ui;

//Usages
use crate::network_initializer::NetworkInit;

fn main() {
    println!("Hello, world!");
    let mut myNet = NetworkInit::new();
    myNet.parse("input.toml")
}
