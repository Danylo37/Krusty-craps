
//UI made by lillo since CHen can't code

use std::io;
use wg_2024::network::NodeId;
use crate::simulation_controller::SimulationController;
use crate::general_use::{ClientCommand, ClientType, ServerType};

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
            2 => { crash_drone(&mut controller) },
            3 => break, //we break from the loop, thus we exit from the interaction.
            _ => println!("Not a valid option, choose again"),
        }
    }
}

fn crash_drone(controller: &mut SimulationController) {
    println!("Enter the ID of the drone to crash:");

    let mut input = String::new();

    io::stdin().read_line(&mut input).expect("Failed to read input");


    let drone_id: NodeId = match input.trim().parse() {
        Ok(id) => id,
        Err(_) => {
            println!("Invalid input. Please enter a valid drone ID.");
            return; // Or handle the error differently (e.g., loop until valid input)
        }
    };

    match controller.request_drone_crash(drone_id) {
        Ok(()) => println!("Crash command sent to drone {}", drone_id),
        Err(e) => println!("Error: {}", e),  // Display the specific error returned by request_drone_crash
    }
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
        println!("Error in parse: {} \n Try again \n", e);
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


fn use_clients(controller: &mut SimulationController) {
    println!("\nAvailable Clients:\n");

    let clients_with_types = controller.get_list_clients_with_types();  // Get clients with types
    if clients_with_types.is_empty() {
        println!("No clients registered.");
        return;
    }

    for (i, (client_type, client_id)) in clients_with_types.iter().enumerate() {
        println!("{}. {} Client with Node ID {}", i + 1, client_type, client_id); // Display type
    }


    let user_choice = ask_input_user();

    if let Some((client_type, client_id)) = clients_with_types.get((user_choice - 1) as usize) { // Check valid index
        choose_action_client(*client_type, *client_id, controller); // Pass client_type
    } else {
        println!("Invalid client choice.");
    }

}

fn choose_action_client(client_type: ClientType, client_id: NodeId, controller: &mut SimulationController) {

    //Variable that allows to go back
    let mut stay_inside = true;
    while stay_inside {

        ///Choosing client function
        println!("\nChoose action for {} Client {}:", client_type, client_id);

        match client_type {    // Different options for Web and Chat clients
            ClientType::Chat => {
                println!(
                    "1. Start Flooding\n
                    2. Ask Server Something\n
                    3. Go Back"); // Chat client options

                let user_choice = ask_input_user();
                match user_choice {
                    1 => {
                        if let Err(e) = controller.start_flooding_on_client(client_id) {
                            eprintln!("Error starting flooding: {}", e);
                        }
                    },
                    2 => ask_server_action(client_id, controller),
                    3 => stay_inside = false,
                    _ => println!("Invalid choice."),
                }
            }

            ClientType::Web => {
                println!("1. Get list of servers");
                println!("2. Request Text from Text Server");
                println!("3. Request Media from Media Server");

                println!("4. Go Back");

                let user_choice = ask_input_user();
                match user_choice {
                    1 => {
                        //TODO
                    }

                    2 => {
                        //TODO
                    }
                    3 => {
                        //TODO
                    }

                    4 => stay_inside = false,
                    _ => println!("Invalid choice."),

                }
            }
        }
    }
}

fn ask_server_action(client_id: NodeId, controller: &mut SimulationController) {

    ///!!! To make with more servers

    //Variable that allows to go back
    let mut stay_inside = true;
    while stay_inside {

        println!("\nAvailable Servers:\n");

        let server_ids_types = controller.get_list_servers_with_types(); // Get servers with types
        if server_ids_types.is_empty() {
            println!("No servers registered.");
            stay_inside = false; // Or return to previous menu
        }

        for (i, (server_type, server_id)) in server_ids_types.iter().enumerate() {
            println!("{}. {} Server with ID {}", i + 1, server_type, server_id);
        }


        println!("\nChoose a server (or 0 to go back):");
        let user_choice = ask_input_user();


        if user_choice == 0 { //Go back
            stay_inside = false;
        }


        if let Some((server_type, server_id)) = server_ids_types.get((user_choice - 1) as usize) { // Get server_type

            match controller.ask_which_type(client_id, *server_id) {
                Ok(server_type) => println!("Server type is: {}", server_type),  // Use returned server_type
                Err(e) => println!("Error getting server type: {}", e),
            }


        } else {
            println!("Invalid server choice.");

        }

    }
}

fn request_text_from_server(client_id: NodeId, server_list: &Vec<(ServerType, NodeId)>, controller: &mut SimulationController){
    for (i, &(_, server_id)) in server_list.iter().enumerate() {
        println!("{}. Text server with ID {}", i + 1, server_id);
    }

    println!("\nChoose a server:");
    let user_choice = ask_input_user();

    if let Some(&(_, server_id)) = server_list.get((user_choice - 1) as usize) {
        if let Some(client_sender) = controller.command_senders_clients.get(&client_id) {
            if let Err(e) = client_sender.send(ClientCommand::RequestText(server_id)) { // Correct command
                eprintln!("Failed to send RequestText command: {:?}", e);
            }
        }
    }

}



fn request_media_from_server(client_id: NodeId, server_list: &Vec<(ServerType, NodeId)>, controller: &mut SimulationController){
    for (i, &(_, server_id)) in server_list.iter().enumerate() {
        println!("{}. Media server with ID {}", i + 1, server_id);
    }

    println!("\nChoose a server:");
    let user_choice = ask_input_user();

    if let Some(&(_, server_id)) = server_list.get((user_choice - 1) as usize) {
        if let Some(client_sender) = controller.command_senders_clients.get(&client_id) {
            if let Err(e) = client_sender.send(ClientCommand::RequestMedia(server_id)) { // Correct command
                eprintln!("Failed to send RequestMedia command: {:?}", e);
            }
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
