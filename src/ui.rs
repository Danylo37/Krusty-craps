use std::collections::HashMap;
//UI made by lillo since CHen can't code
use crate::clients::client_chen::ClientChen;
use crate::general_use::*;
use crate::network_initializer::NetworkInit;
use crate::server::{CommunicationServer, ContentServer};
use std::io;
use crossbeam_channel::{Receiver, Sender};
use wg_2024::network::NodeId;
use wg_2024::packet::NodeType::Client;
use wg_2024::packet::Packet;
use crate::simulation_controller::SimulationController;

pub fn start_ui(mut controller: SimulationController) {
    loop {

        ///Choosing base options
        println!(
"Choose an option
1. Use clients
2. Crashing a drone
3. Nothing"
        );
        let user_choice = ask_input_user();

        match user_choice {
            1 => { use_clients(&mut controller) }
            2 => { crash_drone(&mut controller) }, //you need to put the crash of the drone function
            3 => break, //we break from the loop, thus we exit from the interaction.
            _ => println!("Not a valid option, choose again"),
        }
    }
}

fn crash_drone(controller: &mut SimulationController) {
    todo!()
}

fn ask_input_user() -> i32 {
    loop {
        let user_input = take_user_input_and_parse();
        if user_input != -1 {
            return user_input;
        }
    }

}
fn take_user_input_and_parse() -> i32 {
    let mut user_input = String::new();
    io::stdin() //get the input from the keyboard, in this case I expect a number
        .read_line(&mut user_input)
        .expect("Error in reading your choice");
    user_input.trim().parse().unwrap_or_else(|e| {
        println!("Error in parse: {}", e);
        -1
    })
}

//Maybe useful later
fn take_user_input() -> String {
    let mut user_input = String::new();
    io::stdin() //get the input from the keyboard, in this case I expect a number
        .read_line(&mut user_input)
        .expect("Error in reading your choice");
    user_input.trim().to_string()
}


fn use_clients(controller: &mut SimulationController){

    println!("\n Which client? \n");

    //Print clients, controller.get_list_clients() function that returns a vec<NodeId>
    let clients_ids = controller.get_list_clients();
    for (i, client) in clients_ids.iter().enumerate(){
        println!(
            "{}- Client with nodeId {} \n", i+1, client
        );
    }

    let user_choice = ask_input_user();
    let client_id_chose = clients_ids[(user_choice-1) as usize];
    ///We should do a check if the id user chose exists!!!

    choose_action_client(client_id_chose, controller);
}

fn choose_action_client(client_id_chose: NodeId, controller: &mut SimulationController) {

    //Variable that allows to go back
    let mut stay_inside = true;
    while stay_inside {

        ///Choosing client function
        println!("\n\n Choose client function?");
        println!(
"1. Start flooding
2. Ask the server something
3. Go back"
        );
        //2 is to change with more servers

        let user_choice = ask_input_user();


        match user_choice {
            1 => { /*controller.start_flooding_on_client(client_id);  to do*/ }
            2 => {ask_server_action(client_id_chose, controller)}
            3 => { stay_inside = false; }
            _ => println!("Not a valid option, choose again")
        }
    }
}

fn ask_server_action(client_id_chose: NodeId, controller: &mut SimulationController) {

    ///!!! To make with more servers

    //Variable that allows to go back
    let mut stay_inside = true;
    while stay_inside {

        ///Choosing what to ask server
        println!("\n\n What is your query?");
        println!(
"1. Ask type to the server
2. More
3. Go back"
        );
        let user_choice = ask_input_user();

        match user_choice {
            1 => { controller.ask_server_type_with_client_id(client_id_chose, 4).unwrap() } //For testing it doesn't choose the server, it's only one with NodeId 4
            2 => println!("to do"),
            3 => { stay_inside = false; }
            _ => println!("Not a valid option, choose again")
        }

    }
}

fn ask_comm_server(client_id_chose: NodeId, sever_id_chose: NodeId, controller: &mut SimulationController) {
    /*
    println!("What you wanna ask to the communication server:");
    println!("1. Add a client into the list");
    println!("2. Ask for the list of all the registered clients");
    println!("3. Send message to a client");
    println!("4. I have nothing to do with this server, get me out of here");
*/
}
