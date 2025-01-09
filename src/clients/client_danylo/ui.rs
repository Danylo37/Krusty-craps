use std::io::{self, Write};
use wg_2024::network::NodeId;
use crate::general_use::ServerType;
use super::client_danylo::ChatClientDanylo;


pub struct ChatClientUI<'a> {
    client: &'a mut ChatClientDanylo,
    current_server: NodeId,
    separator: String
}

impl<'a> ChatClientUI<'a> {
    /// ###### Creates a new `ChatClientUI` instance with the given client.
    pub fn new(client: &'a mut ChatClientDanylo) -> Self {
        Self {
            client,
            current_server: 0,
            separator: "-------------------------".to_string(),
        }
    }

    /// ###### Starts the user interface for the chat client.
    pub fn run(&mut self) {
        // Display the initial menu
        self.display_main_menu();

        loop {
            // Get user input
            let user_choice = Self::ask_input_user();

            // Exit condition
            if user_choice == 0 {
                println!("Exiting...");
                break;
            }

            // Handle user choices based on topology state
            match (self.client.topology.is_empty(), user_choice) {
                (true, 1) => self.discover_network(),
                (false, 1) => self.check_inbox(),
                (false, 2) => self.send_request(),
                _ => {
                    println!("Invalid option. Please try again.");
                    continue;
                },
            }

            // Display the menu after processing
            self.display_main_menu();
        }
        println!("{}", self.separator);
    }

    /// ###### Displays the main menu for the chat client.
    /// The options depend on whether the client's topology is empty.
    fn display_main_menu(&self) {
        println!("\n------- Client {} --------", self.client.id);
        if self.client.topology.is_empty() {
            println!("1. Discover the network");
        } else {
            println!("1. Check inbox");
            println!("2. Send request to server");
        }
        println!("0. Exit");
    }

    /// ###### Prompts the user for input and ensures valid numeric input is returned.
    fn ask_input_user() -> usize {
        loop {
            print!("> ");
            io::stdout().flush().unwrap();

            match Self::take_user_input_and_parse() {
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
    fn discover_network(&mut self) {
        println!("{}\n", self.separator);
        println!("------- Discovery -------");
        println!("Starting discovery...");
        match self.client.discovery() {
            Ok(_) => {
                println!("Discovery complete!");
            }
            Err(error) => {
                println!("Discovery failed: {}", error);
            }
        }
        Self::only_back_option();
        println!("{}", self.separator);
    }

    /// ###### Displays the inbox for the client and resets the new message counter.
    /// If the inbox is empty, notifies the user. Otherwise, displays each message.
    fn check_inbox(&self) {
        println!("{}\n", self.separator);
        println!("--------- Inbox ---------");

        // Display messages or notify if inbox is empty
        if self.client.inbox.is_empty() {
            println!("No messages yet.");
        } else {
            for (sender, message) in &self.client.inbox {
                println!("Message from client {}:", sender);
                println!("{}", message);
                println!("- - - - - - - - - - - - -")
            }
        }

        Self::only_back_option();
        println!("{}", self.separator);
    }

    /// ###### Displays a "Back to main menu" option and waits for the user to select it.
    /// Ensures the user cannot proceed without selecting the correct option.
    fn only_back_option() {
        println!("0. Back to main menu");

        loop {
            let user_choice = Self::ask_input_user();

            if user_choice == 0 {
                break;
            } else {
                println!("Invalid option. Please try again.");
            }
        }
    }

    /// ###### Allows the user to select a server and interact with it.
    /// Displays the list of servers and handles user input.
    fn send_request(&mut self) {
        let mut servers: Vec<(NodeId, ServerType)> = self.client.servers.clone().into_iter().collect();
        servers.sort_by_key(|k| k.0);

        self.display_choose_server(&servers);

        loop {
            let user_choice = Self::ask_input_user();

            if user_choice == 0 {
                break;
            } else if (1..=servers.len()).contains(&user_choice) {
                let (server_id, server_type) = &servers[user_choice - 1];
                self.current_server = *server_id;
                self.server_menu(server_type);
                break;
            } else {
                println!("Invalid option. Please try again.");
            }
        }
        println!("{}", self.separator);
    }

    /// ###### Displays the list of available servers for the user to choose from.
    /// If no servers are found, notifies the user.
    fn display_choose_server(&self, servers: &Vec<(NodeId, ServerType)>) {
        println!("{}\n", self.separator);
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
    fn server_menu(&mut self, server_type: &ServerType) {
        match server_type {
            ServerType::Communication => {
                self.communication_server_menu();
            }
            ServerType::Text | ServerType::Media => {
                self.what_are_you_doing_menu();
            }
            ServerType::Undefined => {
                self.undefined_server_menu();
            }
        };
    }

    /// ###### Handles the communication server menu.
    /// Allows the user to interact with the server based on their registration status.
    fn communication_server_menu(&mut self) {
        // Check if the client is registered on the server
        let is_registered = *self.client.is_registered.get(&self.current_server).unwrap();

        self.display_communication_server_options(is_registered);

        loop {
            let user_choice = Self::ask_input_user();

            match user_choice {
                0 => break,
                1 if is_registered => {
                    self.request_sending("AskListClients");
                    break;
                }
                2 if is_registered => {
                    self.send_message();
                    break;
                }
                1 => {
                    self.request_sending("RegisterClient");
                    break;
                }
                _ => {
                    println!("Invalid option. Please try again.");
                }
            }
        }
    }

    /// ###### Displays the available options for the communication server menu based on the client's registration status.
    fn display_communication_server_options(&self, is_registered: bool) {
        println!("{}\n", self.separator);
        println!("------ Comm. server -----");

        match is_registered {
            true => {
                println!("1. Request client's list");
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
    fn what_are_you_doing_menu(&self) {
        println!("{}\n", self.separator);
        println!("---------- ??? ----------");
        println!("Bro, chill, you are not a web browser xD");
        Self::only_back_option();
    }

    /// ###### Displays the undefined server menu and allows the user to interact with the server.
    /// The user can request the server type or go back to the main menu.
    fn undefined_server_menu(&mut self) {
        println!("{}\n", self.separator);
        println!("----- Undef. server -----");
        println!("1. Request server type");
        println!("0. Back to main menu");

        loop {
            let user_choice = Self::ask_input_user();

            match user_choice {
                0 => break,
                1 => {
                    self.request_sending("AskType");
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
    fn request_sending(&mut self, query: &str) {
        println!("{}\n", self.separator);
        println!("---- Request sending ----");

        match query {
            "AskType" => {
                self.handle_ask_type();
            }
            "RegisterClient" => {
                self.handle_register_client();
            }
            "AskListClients" => {
                self.handle_ask_users_list()
            }
            _ => {}
        }
        Self::only_back_option();
    }

    /// ###### Handles the request to get the server type.
    fn handle_ask_type(&mut self) {
        println!("Requesting server type...");
        let server_id = self.current_server;

        match self.client.request_server_type(server_id) {
            Ok(_) => {
                println!("Server type is: {}", self.client.servers.get(&server_id).unwrap());
            }
            Err(error) => {
                println!("Failed to get server type: {}", error);
            }
        };
    }

    /// ###### Handles the request to register a client to the server.
    fn handle_register_client(&mut self) {
        println!("Requesting to register...");

        match self.client.request_to_register(self.current_server) {
            Ok(_) => {
                println!("You have registered successfully!");
            }
            Err(error) => {
                println!("Failed to register: {}", error);
            }
        }
    }

    /// ###### Handles the response from the server after requesting the list of clients.
    fn handle_ask_users_list(&mut self) {
        println!("Requesting list of clients...");

        match self.client.request_users_list(self.current_server) {
            Ok(_) => {
                println!("Client list:");
                for user_id in self.client.clients.iter() {
                    println!("Client {}", user_id);
                }
            }
            Err(error) => {
                println!("Failed to get clients list: {}", error);
            }
        };
    }

    /// ###### Selects a client to send a message to and handles the message sending process.
    fn send_message(&mut self) {
        let mut clients: Vec<NodeId> = self.client.clients.clone().into_iter().collect();
        clients.sort();

        self.display_choose_user(&clients);

        loop {
            let user_choice = Self::ask_input_user();

            if user_choice == 0 {
                break;
            } else if (1..=clients.len()).contains(&user_choice) {
                self.message_sending(clients[user_choice - 1]);
                break;
            } else {
                println!("Invalid option. Please try again.");
            }
        }
    }

    /// ###### Displays a list of clients for the sender to choose from.
    fn display_choose_user(&self, clients: &Vec<NodeId>) {
        println!("{}\n", self.separator);
        println!("----- Choose client -----");

        if clients.is_empty() {
            println!("No clients found.");
        } else {
            for (index, user_id) in clients.iter().enumerate() {
                println!("{}. Client {}", index + 1, user_id);
            }
        }

        println!("0. Back to main menu");
    }

    /// ###### Sends a message to the selected client.
    fn message_sending(&mut self, recipient: NodeId) {
        println!("{}\n", self.separator);
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

                    match self.client.send_message_to(recipient, message.to_string(), self.current_server) {
                        Ok(_) => {
                            println!("Message sent successfully!");
                        }
                        Err(error) => {
                            println!("Failed to send message: {}", error);
                        }
                    }
                }
            }
            Err(error) => println!("Error reading input: {}", error),
        }

        Self::only_back_option();
    }

}
