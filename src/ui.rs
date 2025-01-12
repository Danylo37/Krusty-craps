
//UI made by lillo since CHen can't code

use std::io;
use wg_2024::network::NodeId;
use crate::simulation_controller::SimulationController;
use crate::general_use::{ClientCommand, ClientType, ServerType};

pub fn start_ui(mut controller: SimulationController) {
    loop {

        //Choosing base options
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

fn ask_input_user() -> usize {
    loop {
        let user_input = take_user_input_and_parse();
        if user_input != usize::MAX { //usize::MAX is the error value
            return user_input;
        }
    }

}
fn take_user_input_and_parse() -> usize {
    let mut user_input = String::new();
    io::stdin() //get the input from the keyboard, in this case I expect a number
        .read_line(&mut user_input)
        .expect("Error in reading your choice");
    user_input.trim().parse().unwrap_or_else(|e| {
        println!("Error in parse: {} \n Try again \n", e);
        usize::MAX
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


    let clients_with_types = controller.get_list_clients();
    if clients_with_types.is_empty() {
        println!("No clients registered.");
        return;
    }

    let mut client_options = clients_with_types.clone(); // Clone to sort
    client_options.sort_by_key(|(_, id)| *id);



    for (i, (client_type, client_id)) in client_options.iter().enumerate() {
        println!("{}. {} Client with Node ID {}", i + 1, client_type, client_id); // Display type
    }



    let user_choice = ask_input_user();

    if let Some((client_type, client_id)) = client_options.get(user_choice - 1) {
        choose_server(*client_type, *client_id, controller);  // Choose server right after selecting the client
    } else {
        println!("Invalid client choice.");
    }
}

fn choose_server(client_type: ClientType, client_id: NodeId, controller: &mut SimulationController) {
    println!("\nRequesting known servers for client {}...", client_id);

    // Request the list of known servers from the client
    if let Err(e) = controller.request_known_servers(client_id) {
        eprintln!("Error requesting known servers: {}", e);
        return;
    }

    loop {  // Loop for server selection
        println!("\nChoose action for {} Client {}:", client_type, client_id);


        let servers_with_types = controller.get_list_servers();



        if servers_with_types.is_empty(){
            println!("No servers found. Press Enter to discover.");


            let mut input = String::new();

            io::stdin().read_line(&mut input).expect("Failed to read input");

            controller.start_flooding_on_client(client_id).expect("panic message");
            continue;


        }


        for (i, (server_type, server_id)) in servers_with_types.iter().enumerate() {
            println!("{}. {} Server with ID {}", i + 1, server_type, server_id);
        }
        println!("\nChoose a server (or 0 to go back):");


        let user_choice = ask_input_user();


        if let Some((server_type, server_id)) = servers_with_types.get(user_choice - 1) {

            match *server_type{
                ServerType::Communication => ask_comm_server(client_id, *server_id, controller),

                ServerType::Text => {/*Request text*/},  // Placeholder for text server action        TODO
                ServerType::Media => {/*Request media*/}, // Placeholder for media server action        TODO
                _=> println!("Cannot send request to undefined server!")
            }

        } else if user_choice == 0 { // Go back option
            return; // Go back to client selection
        } else {
            println!("Invalid server choice.");

        }


    }



}

fn choose_action_client(client_type: ClientType, client_id: NodeId, controller: &mut SimulationController) {

    loop { // Loop for easier control flow

        //Choosing client function
        println!("\nChoose action for {} Client {}:", client_type, client_id);

        let servers = controller.get_list_servers();
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
                    2 => {
                        ask_server_action(client_id, servers, controller); // Pass server list to ask_server_action
                    },
                    3 => break, // Exit the loop and go back
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
                        for (i, (server_type, server_id)) in servers.iter().enumerate() {
                            println!("{}. {} Server with ID {}", i + 1, server_type, server_id);
                        }
                    }

                    2 => {
                        let text_servers: Vec<_> = servers.iter().filter(|(server_type, _)| *server_type == ServerType::Text ).cloned().collect();
                        request_text_from_server(client_id, &text_servers, controller); //Request text
                    }
                    3 => {
                        let media_servers: Vec<_> = servers.iter().filter(|(server_type, _)| *server_type == ServerType::Media ).cloned().collect();
                        request_media_from_server(client_id, &media_servers, controller); //Request media
                    }

                    4 => break, // Exit the loop and go back
                    _ => println!("Invalid choice."),

                }
            }
        }
    }
}

fn ask_server_action(client_id: NodeId, server_options: Vec<(ServerType, NodeId)> , controller: &mut SimulationController) {
    loop {
        println!("\nAvailable Servers:\n");
        for (i, (server_type, server_id)) in server_options.iter().enumerate() {
            println!("{}. {} Server with ID {}", i + 1, server_type, server_id);
        }

        println!("\nChoose a server (or 0 to go back):");
        let user_choice = ask_input_user();

        if user_choice == 0 {
            break; // Go back to client menu
        }

        if let Some((server_type, server_id)) = server_options.get(user_choice - 1) {

            match *server_type{
                ServerType::Communication => ask_comm_server(client_id, *server_id, controller),
                ServerType::Text => request_text_from_server(client_id, &server_options, controller), // Example
                ServerType::Media => request_media_from_server(client_id, &server_options, controller),  // Example
                _ => println!("Cannot send request to undefined server!"),
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

    if let Some(&(_, server_id)) = server_list.get(user_choice - 1) {
        if let Some((client_sender, _)) = controller.command_senders_clients.get(&client_id) {
            if let Err(e) = client_sender.send(ClientCommand::RequestText(server_id)) {
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

    if let Some(&(_, server_id)) = server_list.get(user_choice - 1) {
        if let Some((client_sender, _)) = controller.command_senders_clients.get(&client_id) {
            if let Err(e) = client_sender.send(ClientCommand::RequestMedia(server_id)) {
                eprintln!("Failed to send RequestMedia command: {:?}", e);
            }
        }
    }

}

fn ask_comm_server(client_id_chose: NodeId, sever_id_chose: NodeId, controller: &mut SimulationController) {
    /*
    println!("What you want to ask the communication server:");
    println!("1. Add a client into the list");
    println!("2. Ask for the list of all the registered clients");
    println!("3. Send message to a client");
    println!("4. I have nothing to do with this server, get me out of here");
*/
}
