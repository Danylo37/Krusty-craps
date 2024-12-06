//simple ui suggested by lillo in the telegram
use std::io;
pub fn interface() {
    loop {
        println!("What you want to do");
        println!("1. Use the client");
        println!("2. Crashing a drone");
        println!("3. Nothing");

        let mut user_choice = String::new();
        io::stdin()
            .read_line(&mut user_choice)
            .expect("Error in reading your choice");

        let user_choice: u32 = match user_choice.trim().parse() {
            Ok(num) => num,
            Err(_) => continue,
        };

        match user_choice {
            1 => {
                println!("You choose to use client.");
                println!("Then what you wanna do?");
                println!("1. Start flooding");
                println!("2. Ask server something");

                let mut user_choice = String::new();
                io::stdin()   //get the input from the keyboard, in this case I expect a number
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
                        let mut user_question = String::new();
                        io::stdin()
                            .read_line(&mut user_question)
                            .expect("Error in reading your question");
                        println!("You asked :{}", user_question);
                    },
                    _ => println!("Not a valid option, choose again"),
                }
            }
            2 => println!("You crashed the drone"), //you need to put the crash of the drone function
            3 => break,   //we break from the loop, thus we exit from the interaction.
            _ => println!("Not a valid option, choose again"),

        }
    }
}
