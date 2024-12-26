use std::{io, io::Write};
use wg_2024::network::NodeId;
use crate::general_use::ServerType;
use super::client_danylo::ChatClientDanylo;

/// ###### Starts the user interface for the chat client.
/// Handles user input and manages the main interaction loop.
pub fn run_chat_client_ui(mut client: ChatClientDanylo) {
    // Display the initial menu
    display_main_menu(&client);

    loop {
        // Get user input
        let user_choice = ask_input_user();

        // Exit condition
        if user_choice == 0 {
            println!("Exiting...");
            break;
        }

        // Handle user choices based on topology state
        match (client.topology.is_empty(), user_choice) {
            (true, 1) => discover_network(&mut client),
            (false, 1) => check_inbox(&mut client),
            (false, 2) => send_request(&mut client),
            _ => {
                println!("Invalid option. Please try again.");
                continue;
            },
        }

        // Display the menu after processing
        display_main_menu(&client);
    }
    println!("--------------------------");
}

/// ###### Displays the main menu for the chat client.
/// The options depend on whether the client's topology is empty.
fn display_main_menu(client: &ChatClientDanylo) {
    println!("\n------- Client {} --------", client.id);
    if client.topology.is_empty() {
        println!("1. Discover the network");
    } else {
        println!("1. Check inbox ({})", client.new_messages);
        println!("2. Send request to server");
    }
    println!("0. Exit");
}

/// ###### Prompts the user for input and ensures valid numeric input is returned.
fn ask_input_user() -> usize {
    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        match take_user_input_and_parse() {
            Some(input) => return input,
            None => {
                println!("Invalid input. Please try again.");
            }
        }
    }
}

/// ###### Reads user input from stdin and attempts to parse it into a `usize`.
fn take_user_input_and_parse() -> Option<usize> {
    let mut user_input = String::new();
    if let Err(err) = io::stdin().read_line(&mut user_input) {
        eprintln!("Error reading input: {}", err);
        return None;
    }

    user_input.trim().parse().ok()
}

/// ###### Initiates the network discovery process for the client.
fn discover_network(client: &mut ChatClientDanylo) {
    println!("--------------------------\n");
    println!("-------- Discovery --------");
    println!("Starting discovery...");
    client.discovery();
    println!("--------------------------");
}

/// ###### Displays the inbox for the client and resets the new message counter.
/// If the inbox is empty, notifies the user. Otherwise, displays each message.
fn check_inbox(client: &mut ChatClientDanylo) {
    // Reset the new message counter
    client.new_messages = 0;

    println!("-------------------------\n");
    println!("--------- Inbox ---------");

    // Display messages or notify if inbox is empty
    if client.inbox.is_empty() {
        println!("No messages yet.");
    } else {
        for message in &client.inbox {
            println!("Message from {}:", message.0);
            println!("{}", message.1);
            println!()
        }
    }

    only_back_option();
    println!("-------------------------");
}

/// ###### Displays a "Back to main menu" option and waits for the user to select it.
/// Ensures the user cannot proceed without selecting the correct option.
fn only_back_option() {
    println!("0. Back to main menu");

    loop {
        let user_choice = ask_input_user();

        if user_choice == 0 {
            break;
        } else {
            println!("Invalid option. Please try again.");
        }
    }
}

/// ###### Allows the user to select a server and interact with it.
/// Displays the list of servers and handles user input.
fn send_request(client: &mut ChatClientDanylo) {
    let servers = client.servers.clone();

    display_choose_server(&servers);

    loop {
        let user_choice = ask_input_user();

        if user_choice == 0 {
            break;
        } else if (1..=servers.len()).contains(&user_choice) {
            server_menu(&servers[user_choice - 1], client);
            break;
        } else {
            println!("Invalid option. Please try again.");
        }
    }
    println!("-------------------------");
}

/// ###### Displays the list of available servers for the user to choose from.
/// If no servers are found, notifies the user.
fn display_choose_server(servers: &Vec<(NodeId, ServerType)>) {
    println!("-------------------------\n");
    println!("----- Choose server -----");

    if servers.is_empty() {
        println!("No servers found.");
    } else {
        for (index, (id, server_type)) in servers.iter().enumerate() {
            println!("{}. {} server with ID {}", index + 1, server_type, id);
        }
    }

    println!("0. Back to main menu");
}

/// ###### Handles the server menu based on the server type.
/// Dispatches to the appropriate menu or action for the given server type.
fn server_menu(server: &(NodeId, ServerType), client: &mut ChatClientDanylo) {
    let server_id = server.0;
    let server_type = &server.1;

    match server_type {
        ServerType::Communication => {
            communication_server_menu(server_id, client);
        }
        ServerType::Text | ServerType::Media => {
            what_are_you_doing_menu();
        }
        ServerType::Undefined => {
            undefined_server_menu(server_id, client);
        }
    };
}

/// ###### Handles the communication server menu.
/// Allows the user to interact with the server based on their registration status.
fn communication_server_menu(server_id: NodeId, client: &mut ChatClientDanylo) {
    // Check if the client is registered on the server
    let is_registered = *client.is_registered.get(&server_id).unwrap();

    display_communication_server_options(is_registered);

    loop {
        let user_choice = ask_input_user();

        match user_choice {
            0 => break,
            1 if is_registered => {
                request_sending(server_id, client, "AskListClients");
                break;
            }
            2 if is_registered => {
                send_message(client, server_id);
                break;
            }
            1 => {
                request_sending(server_id, client, "AddClient");
                break;
            }
            _ => {
                println!("Invalid option. Please try again.");
            }
        }
    }
}

/// ###### Displays the available options for the communication server menu
/// based on the user's registration status.
fn display_communication_server_options(is_registered: bool) {
    println!("-------------------------\n");
    println!("------ Comm. server -----");

    match is_registered {
        true => {
            println!("1. Request user's list");
            println!("2. Send message");
        }
        false => {
            println!("1. Register");
        }
    }

    println!("0. Back to main menu");
}

/// ###### Displays a menu when the user tries to perform an action not supported by the server.
/// Gives a humorous message and allows the user to go back.
fn what_are_you_doing_menu() {
    println!("-------------------------\n");
    println!("---------- ??? ----------");
    println!("Bro, chill, you are not a web browser xD");
    only_back_option();
}

/// ###### Displays the undefined server menu and allows the user to interact with the server.
/// The user can request the server type or go back to the main menu.
fn undefined_server_menu(server_id: NodeId, client: &mut ChatClientDanylo) {
    println!("-------------------------\n");
    println!("----- Undef. server -----");
    println!("1. Request server type");
    println!("0. Back to main menu");

    loop {
        let user_choice = ask_input_user();

        match user_choice {
            0 => break,
            1 => {
                request_sending(server_id, client, "AskType");
                break;
            }
            _ => {
                println!("Invalid option. Please try again.");
            }
        }
    }
}

/// ###### Sends a request to the server based on the query provided.
/// Waits for the server's response after sending the request.
fn request_sending(server_id: NodeId, client: &mut ChatClientDanylo, query: &str) {
    println!("-------------------------\n");
    println!("---- Request sending ----");

    match query {
        "AskType" => {
            println!("Requesting server type...");
            client.request_server_type(server_id);
        }
        "AddClient" => {
            println!("Registering to register...");
            client.request_to_register(server_id);
        }
        "AskListUsers" => {
            println!("Requesting list of users...");
            client.request_users_list(server_id);
        }
        _ => {}
    }

    client.wait_response();
    only_back_option();
}

/// ###### Allows the user to send a message to another user by selecting from a list of users.
fn send_message(client: &mut ChatClientDanylo, server_id: NodeId) {
    let users = client.users.clone();

    display_choose_user(&users);

    loop {
        let user_choice = ask_input_user();

        if user_choice == 0 {
            break;
        } else if (1..=users.len()).contains(&user_choice) {
            message_sending(users[user_choice - 1], client, server_id);
            break;
        } else {
            println!("Invalid option. Please try again.");
        }
    }
}

/// ###### Displays a list of users for the sender to choose from.
fn display_choose_user(users: &Vec<NodeId>) {
    println!("-------------------------\n");
    println!("------ Choose user ------");

    if users.is_empty() {
        println!("No users found.");
    } else {
        for (index, user_id) in users.iter().enumerate() {
            println!("{}. User {}", index + 1, user_id);
        }
    }

    println!("0. Back to main menu");
}

/// ###### Sends a message to the selected user and handles errors if they occur.
fn message_sending(user: NodeId, client: &mut ChatClientDanylo, server_id: NodeId) {
    println!("-------------------------\n");
    println!("----- Your message ------");

    let mut message = String::new();

    match io::stdin().read_line(&mut message) {
        Ok(_) => {
            // Trim the message to remove unnecessary whitespace
            let message = message.trim();
            if message.is_empty() {
                println!("Message cannot be empty. Please try again.");
            } else {
                println!("Sending message...");
                client.send_message_to(user, message.to_string(), server_id);
            }
        }
        Err(error) => println!("Error reading input: {}", error),
    }

    only_back_option();
}
