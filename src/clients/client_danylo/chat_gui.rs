use std::thread;
use eframe::{
    App,
    Frame,
    egui::{CentralPanel, Context, Ui}
};
use wg_2024::{
    network::NodeId,
    packet::NodeType
};
use crate::general_use::ServerType;
use super::{ChatClientDanylo, Node};

#[derive(PartialEq)]
enum Menu {
    Main,
    Inbox,
    Discover,

    ChooseServer,
    CommunicationServer,
    ContentServer,
    UndefinedServer,
    SendRequest(RequestType),

    ChooseUser,
    SendMessageTo(NodeId),
}

#[derive(PartialEq)]
enum RequestType {
    AskType,
    RegisterClient,
    AskListClients,
}

pub struct ChatGUI<'a> {
    client: &'a mut ChatClientDanylo,
    current_menu: Menu,
    current_server: NodeId,
    current_message: Option<String>,
    current_message_status: Option<String>,
    receive_responses: bool,
    wait_response: bool
}

impl App for ChatGUI<'_>  {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        CentralPanel::default().show(ctx, |ui| {
            match self.current_menu {
                Menu::Main => self.main_menu(ui),
                Menu::Inbox => self.inbox(ui),
                Menu::ChooseServer => self.choose_server(ui),
                Menu::Discover => self.discovery(ui),

                Menu::CommunicationServer => self.communication_server_menu(ui),
                Menu::ContentServer => self.what_are_you_doing(ui),
                Menu::UndefinedServer => self.undefined_server_menu(ui),

                Menu::SendRequest(RequestType::AskListClients) => self.ask_clients_list(ui),
                Menu::SendRequest(RequestType::RegisterClient) => self.register_client(ui),
                Menu::SendRequest(RequestType::AskType) => self.ask_type(ui),

                Menu::ChooseUser => self.choose_user(ui),
                Menu::SendMessageTo(id) => self.send_message(ui, id),
            }
        });
    }
}

impl <'a> ChatGUI<'a> {
    pub fn new(client: &'a mut ChatClientDanylo) -> Self {
        ChatGUI {
            client,
            current_menu: Menu::Main,
            current_server: 0,
            current_message: Some(String::new()),
            current_message_status: None,
            receive_responses: false,
            wait_response: false,
        }
    }

    pub fn run(self) {
        let options = eframe::NativeOptions::default();
        eframe::run_native(
            format!("Chat Client {}", self.client.id).as_str(),
            options,
            Box::new(|_cc| Ok(Box::new(self))),
        ).unwrap();
    }

    fn main_menu(&mut self, ui: &mut Ui) {
        ui.heading("Main menu");
        ui.separator();
        ui.vertical(|ui| {
            if ui.button("Inbox").clicked() {
                self.current_menu = Menu::Inbox;
            }
            if ui.button("Send Request").clicked() {
                self.current_menu = Menu::ChooseServer;
            }
            if ui.button("Discover").clicked() {
                self.current_menu = Menu::Discover;
            }
        });
    }

    fn inbox(&mut self, ui: &mut Ui) {
        ui.heading("Inbox");
        ui.separator();
        ui.vertical(|ui| {
            if self.client.inbox.is_empty() {
                ui.label("No messages yet.");
                ui.separator();
            } else {
                for (sender, message) in &self.client.inbox {
                    ui.label(format!("From client {}:\n{}", sender, message));
                    ui.separator();
                }
            }

            if ui.button("Back").clicked() {
                self.current_menu = Menu::Main;
            }
        });
    }

    fn choose_server(&mut self, ui: &mut Ui) {
        ui.heading("Choose a server");
        ui.separator();

        let mut servers: Vec<(NodeId, ServerType)> = self.client.servers.clone().into_iter().collect();
        servers.sort_by_key(|k| k.0);

        if servers.is_empty() {
            ui.label("No servers found.");
        } else {
            for (index, (id, server_type)) in servers.iter().enumerate() {
                if ui.button(format!("{}. {} server {}", index + 1, server_type, id)).clicked() {
                    self.current_server = *id;
                    self.handle_server_selection(server_type);
                }
            }
        }

        ui.separator();
        if ui.button("Back").clicked() {
            self.current_menu = Menu::Main;
        }
    }

    fn handle_server_selection(&mut self, server_type: &ServerType) {
        match server_type {
            ServerType::Communication => {
                self.current_menu = Menu::CommunicationServer;
            }
            ServerType::Text | ServerType::Media => {
                self.current_menu = Menu::ContentServer;
            }
            ServerType::Undefined => {
                self.current_menu = Menu::UndefinedServer;
            }
        }
    }

    fn communication_server_menu(&mut self, ui: &mut Ui) {
        let is_registered = *self.client.is_registered.get(&self.current_server).unwrap_or(&false);

        ui.heading(format!("Communication Server {}", self.current_server));
        ui.separator();

        if is_registered {
            if ui.button("Request client's list").clicked() {
                self.current_menu = Menu::SendRequest(RequestType::AskListClients);
            }
            if ui.button("Send message").clicked() {
                self.current_menu = Menu::ChooseUser;
            }
        } else {
            if ui.button("Register").clicked() {
                self.current_menu = Menu::SendRequest(RequestType::RegisterClient);
            }
        }

        ui.separator();
        if ui.button("Back").clicked() {
            self.current_menu = Menu::ChooseServer;
        }
    }

    fn what_are_you_doing(&mut self, ui: &mut Ui) {
        ui.heading("???");
        ui.separator();
        ui.label("Bro, chill, I am not a web browser xD");

        ui.separator();
        if ui.button("Back").clicked() {
            self.current_menu = Menu::ChooseServer;
        }
    }

    fn undefined_server_menu(&mut self, ui: &mut Ui) {
        ui.heading("Undefined Server Menu");
        ui.separator();

        if ui.button("Request server type").clicked() {
            self.current_menu = Menu::SendRequest(RequestType::AskType);
        }

        ui.separator();
        if ui.button("Back").clicked() {
            self.current_menu = Menu::ChooseServer;
        }
    }

    fn choose_user(&mut self, ui: &mut Ui) {
        ui.heading("Choose a user");
        ui.separator();

        let mut clients: Vec<NodeId> = self.client.clients.clone().into_iter().collect();
        clients.sort();

        if clients.is_empty() {
            ui.label("No clients found.");
        } else {
            for client_id in clients.iter() {
                if ui.button(format!("Client {}", client_id)).clicked() {
                    self.current_menu = Menu::SendMessageTo(*client_id);
                }
            }
        }

        ui.separator();
        if ui.button("Back").clicked() {
            self.current_menu = Menu::CommunicationServer;
        }
    }

    fn send_message(&mut self, ui: &mut Ui, recipient: NodeId) {
        ui.heading(format!("Send message to client {}", recipient));
        ui.separator();

        let message = self.current_message.as_mut().unwrap();

        ui.horizontal(|ui| {
            ui.label("Your message: ");
            ui.text_edit_singleline(message);
        });

        while self.wait_response {
            if self.client.response_received {
                self.current_message_status = Some("Message delivered successfully!".to_string());
                self.wait_response = false;
            }
            if let Some(error) = &self.client.external_error {
                self.current_message_status = Some(format!("Failed to send message: {}", error));
                self.wait_response = false;
            }
            thread::sleep(std::time::Duration::from_millis(10));
        }

        if self.current_message_status.is_some() {
            ui.label(self.current_message_status.as_ref().unwrap());
        }

        if ui.button("Send").clicked() {

            if message.trim().is_empty() {
                self.current_message_status = Some("Message cannot be empty.".to_string());
            } else {
                match self.client.send_message_to(recipient, message.trim().to_string(), self.current_server) {
                    Ok(_) => self.wait_response = true,
                    Err(error) => self.current_message_status = Some(format!("Failed to send message: {}", error)),
                };
            }
        }

        ui.separator();
        if ui.button("Back").clicked() {
            self.current_message = Some(String::new());
            self.current_message_status = None;
            self.current_menu = Menu::ChooseUser;
        }
    }

    fn ask_type(&mut self, ui: &mut Ui) {
        ui.heading("Request server type");
        ui.separator();
        let server_id = self.current_server;

        ui.label("Requesting server type...");
        match self.client.request_server_type(server_id) {
            Ok(_) => self.wait_response = true,
            Err(error) => {
                self.current_message_status = Some(format!("Failed to get server type: {}", error));
            }
        }

        while self.wait_response {
            if self.client.response_received {
                self.current_message_status = Some(format!("Server type is: {}", self.client.servers.get(&server_id).unwrap()));
                self.wait_response = false;
            }
            if let Some(error) = &self.client.external_error {
                self.current_message_status = Some(format!("Failed to get server type: {}", error));
                self.wait_response = false;
            }
            thread::sleep(std::time::Duration::from_millis(10));
        }

        if self.current_message_status.is_some() {
            ui.label(self.current_message_status.as_ref().unwrap());
        }

        ui.separator();
        if ui.button("Back").clicked() {
            self.current_message_status = None;
            self.current_menu = Menu::ChooseServer;
        }
    }

    fn register_client(&mut self, ui: &mut Ui) {
        ui.heading("Request to register");
        ui.separator();

        ui.label("Requesting to register...");
        match self.client.request_to_register(self.current_server) {
            Ok(_) => self.wait_response = true,
            Err(error) => {
                self.current_message_status = Some(format!("Failed to register: {}", error));
            }
        }

        while self.wait_response {
            if self.client.response_received {
                self.current_message_status = Some("You have registered successfully!".to_string());
                self.wait_response = false;
            }
            if let Some(error) = &self.client.external_error {
                self.current_message_status = Some(format!("Failed to register: {}", error));
                self.wait_response = false;
            }
            thread::sleep(std::time::Duration::from_millis(10));
        }

        if self.current_message_status.is_some() {
            ui.label(self.current_message_status.as_ref().unwrap());
        }

        ui.separator();
        if ui.button("Back").clicked() {
            self.current_message_status = None;
            self.current_menu = Menu::ChooseServer;
        }
    }

    fn ask_clients_list(&mut self, ui: &mut Ui) {
        ui.heading("Request list of clients");
        ui.separator();

        ui.label("Requesting list of clients...");
        match self.client.request_clients_list(self.current_server) {
            Ok(_) => self.wait_response = true,
            Err(error) => {
                self.current_message_status = Some(format!("Failed to get clients list: {}", error));
            }
        }

        while self.wait_response {
            if self.client.response_received {
                self.current_message_status = Some(self.get_clients_string());
                self.wait_response = false;
            }
            if let Some(error) = &self.client.external_error {
                self.current_message_status = Some(format!("Failed to get clients list: {}", error));
                self.wait_response = false;
            }
            thread::sleep(std::time::Duration::from_millis(10));
        }

        if self.current_message_status.is_some() {
            ui.label(self.current_message_status.as_ref().unwrap());
        }

        ui.separator();
        if ui.button("Back").clicked() {
            self.current_message_status = None;
            self.current_menu = Menu::CommunicationServer;
        }
    }

    fn get_clients_string(&self) -> String {
        let mut clients_string = "Client list:\n".to_string();
        for client_id in self.client.clients.iter() {
            clients_string.push_str(&format!("Client {}\n", client_id));
        }
        clients_string
    }

    fn discovery(&mut self, ui: &mut Ui) {
        ui.heading("Discovery");
        ui.separator();

        ui.label("Starting discovery...");
        match self.client.discovery() {
            Ok(_) => {
                self.receive_responses = true;
            }
            Err(error) => {
                ui.label(format!("Failed to discover: {}", error));
            }
        }

        while self.receive_responses {  // todo add timeout
            for response in &self.client.flood_responses {
                ui.label(Self::get_response_string(response));
            }
        }

        ui.separator();
        if ui.button("Back").clicked() {
            self.receive_responses = false;
            self.current_menu = Menu::Main;
        }
    }

    fn get_response_string(response: &(Node, Vec<Node>)) -> String {
        let sender_id = response.0.0;
        let sender_type = match response.0.1 {
            NodeType::Client => "Client",
            NodeType::Drone => "Drone",
            NodeType::Server => "Server",
        };
        let path = response.1.clone();

        format!("Response from {} {} with path: {:?}", sender_type, sender_id, path)
    }
}