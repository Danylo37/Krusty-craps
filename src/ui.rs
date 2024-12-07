use std::collections::HashMap;
//simple ui suggested by lillo in the telegram
use crate::clients::client_chen::ClientChen;
use crate::general_use::*;
use crate::network_initializer::NetworkInit;
use crate::server::{CommunicationServer, ContentServer};
use std::io;
use crossbeam_channel::{Receiver, Sender};
use wg_2024::network::NodeId;
use wg_2024::packet::NodeType::Client;
use wg_2024::packet::Packet;

pub fn interface() {
    loop {
        //you as a client

        //the communication server

        println!("What you want to do");
        println!("1. Use the client");
        println!("2. Crashing a drone");
        println!("3. Nothing");

        let mut user_choice = String::new();
        io::stdin()
            .read_line(&mut user_choice)
            .expect("Error in reading your choice");

        let user_choice_id: u32 = match user_choice.trim().parse() {
            Ok(num) => num,
            Err(_) => continue,
        };

        match user_choice_id {
            1 => {
                println!("You choose to use client.");
                println!("Then what you wanna do?");
                println!("1. Start flooding");
                println!("2. Ask server something");
                println!("3. Quit");

                let mut user_choice = String::new();
                io::stdin() //get the input from the keyboard, in this case I expect a number
                    .read_line(&mut user_choice)
                    .expect("Error in reading your choice");

                let user_choice_id: u32 = match user_choice.trim().parse() {
                    Ok(num) => num,
                    Err(_) => continue,
                };

                match user_choice_id {
                    1 => println!("Starting flooding..."),
                    2 => {
                        println!("What you wanna ask to the server?");
                        println!("1. Ask type of the server");
                        println!("2. Quit");
                        let mut user_choice = String::new();
                        io::stdin()
                            .read_line(&mut user_choice)
                            .expect("Error in reading your choice");

                        let user_choice_id: u32 = match user_choice.trim().parse() {
                            Ok(num) => num,
                            Err(_) => continue,
                        };
                        match user_choice_id {
                            1 => {
                                println!("Which type of server:");
                                println!("1. Content server");
                                println!("2. Communication server");
                                println!("3. Get out of here");
                                let mut user_choice = String::new();
                                io::stdin() //get the input from the keyboard, in this case I expect a number
                                    .read_line(&mut user_choice)
                                    .expect("Error in reading your choice");

                                let user_choice_id: u32 = match user_choice.trim().parse() {
                                    Ok(num) => num,
                                    Err(_) => continue,
                                };
                                match user_choice_id{
                                    1 => {
                                        println!("You wanna get some media?");
                                        println!("1. Yes");
                                        println!("2. No, then get me out of here");
                                        let mut user_choice = String::new();
                                        io::stdin() //get the input from the keyboard, in this case I expect a number
                                            .read_line(&mut user_choice)
                                            .expect("Error in reading your choice");

                                        let user_choice_id: u32 = match user_choice.trim().parse() {
                                            Ok(num) => num,
                                            Err(_) => continue,
                                        };
                                        match user_choice_id {
                                            1 => println!("Name of the media, bla, bla"),  //to be completed
                                            2 => break,
                                            _ => println!("Invalid option"),
                                        }
                                    }
                                    2 => {
                                        println!("What you wanna ask to the communication server:");
                                        println!("1. Add a client into the list");
                                        println!("2. Ask for the list of all the registered clients");
                                        println!("3. Send message to a client");
                                        println!("4. I have nothing to do with this server, get me out of here");
                                        let mut user_choice = String::new();
                                        io::stdin() //get the input from the keyboard, in this case I expect a number
                                            .read_line(&mut user_choice)
                                            .expect("Error in reading your choice");

                                        let user_choice_id: u32 = match user_choice.trim().parse() {
                                            Ok(num) => num,
                                            Err(_) => continue,
                                        };
                                        match user_choice_id {
                                            1 => {
                                                println!("Write the client name");
                                                let mut client_name = String::new();
                                                io::stdin().read_line(&mut client_name).expect("Failed!");
                                                println!("Give me the node id");
                                                let mut nodeid = String::new();
                                                io::stdin().read_line(&mut nodeid).expect("Failed!");
                                                let node_id: u32 = match nodeid.trim().parse() {
                                                    Ok(num) => num,
                                                    Err(_) => continue,
                                                };

                                                //need to create the server to really add a client
                                            }

                                            2 => println!("The list:"),
                                            3 => {
                                                println!("Which client you wanna send the message?");
                                                let mut client_name = String::new();
                                                io::stdin().read_line(&mut client_name).expect("Failed!");
                                                println!("What message you wanna send to him/her?");
                                                let mut message = String::new();
                                                io::stdin().read_line(&mut message).expect("Failed!");
                                            }
                                            4 => break,
                                            _ => println!("Invalid option"),
                                        }
                                    }
                                    3 => break,
                                    _ => println!("Not a valid option, choose again"),
                                }
                            }

                            2 => break,
                            _ => println!("Invalid option, choose again"),
                        }
                    }
                    _ => println!("Not a valid option, choose again"),
                }
            }
            2 => println!("You crashed the drone"), //you need to put the crash of the drone function
            3 => break, //we break from the loop, thus we exit from the interaction.
            _ => println!("Not a valid option, choose again"),
        }
    }
}
