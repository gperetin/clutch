extern crate chrono;
extern crate hipchat_client;
extern crate serde;
extern crate termion;
extern crate toml;

#[macro_use]
extern crate serde_derive;

use std::thread;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, Write, stdout, Stdout};
use std::path::Path;

use chrono::prelude::*;

use hipchat_client::Client as HipchatClient;
use hipchat_client::message::Message;
use termion::{color, style, async_stdin};
use termion::raw::IntoRawMode;

const DEFAULT_CONFIG_PATH: &str = "clutch.toml";

#[derive(Deserialize)]
struct Config {
    token: String,
    origin: String,
    room: String,
    user: u64
}

fn setup_client(config: &Config) -> HipchatClient {
    // We use clones because String is allocated on heap, doesn't have
    // a Copy trait, which means it wouldn't be usable after this call.
    // Consider removing .clone() calls if we don't needs these strings
    // after this call
    // If HipchatClient can take a reference (&String), consider passing
    // a reference here. That's a way to refer to a value without taking
    // ownership. This is called borrowing, and such parameters can't
    // be mutated, unless annotated with &mut
    HipchatClient::new(config.origin.clone(), config.token.clone())
}

fn print_message(message: Message) -> String {
    let parsed_date = DateTime::parse_from_rfc3339(&message.date);
    let from: String = message.from.map(|x| x.name).unwrap_or("Unknown".to_string());
    format!("[{}] [{}]: {}",
             parsed_date.unwrap().with_timezone(&Local).format("%m/%d %H:%M").to_string(),
             from,
             message.message)
}

fn prompt_for_message() -> String {
    print!("Message: ");
    io::stdout().flush();

    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer).unwrap();
    buffer
}

fn load_config(path: String) -> Config {
    let mut contents = String::new();
    File::open(path)
        .unwrap_or_else(|e| panic!("{}", e))
        .read_to_string(&mut contents)
        .unwrap_or_else(|e| panic!("{}", e));

    toml::from_str(&contents).unwrap()
}

fn get_config_path(passed_in: Option<String>) -> String {
    let path = passed_in.unwrap_or(DEFAULT_CONFIG_PATH.to_string());
    Path::new(&path)
        .canonicalize()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

fn render_text(text: &String, stdout: &mut Stdout) -> () {
    write!(stdout, "{}{}{}",
           termion::clear::All,
           termion::cursor::Goto(1,1),
           text);
    write!(stdout, "\r\n ------------------------------------------------------------ ");
    write!(stdout, "\r\n> ");
    stdout.flush().unwrap();
}

fn compute_text(messages: &Vec<Message>) -> String {
    let msgs_to_print = messages.iter().rev().take(15);
    let mut msgs: Vec<String> = msgs_to_print.into_iter().map(|m| print_message(m.clone())).collect();
    msgs.reverse();
    msgs.join("\r\n")
}

fn send_message(message: &mut String, client: &HipchatClient, room: &String) -> () {
    client.send_message(room, message.clone());
    message.clear();
}

fn main() {
    let mut stdout = stdout().into_raw_mode().unwrap();
    let config_path = get_config_path(env::args().nth(1));
    let config = load_config(config_path);
    let client = setup_client(&config);

    let messages = client
        .get_recent_history(&config.room)
        .unwrap()
        .items;

    render_text(&compute_text(&messages), &mut stdout);

    let mut stdin = async_stdin().bytes();
    let mut current_message = String::new();
    loop {
        let b = stdin.next();
        match b {
            None => {
                thread::sleep_ms(10);
                continue;
            },
            Some(res) => match res {
                Ok(c) => {
                    match c {
                        3 => break,
                        13 => {
                            send_message(&mut current_message, &client, &config.room);
                            write!(stdout, "\r{}> ", termion::clear::AfterCursor).unwrap();
                            stdout.flush().unwrap();
                        },
                        18 => {
                            // This is Ctrl-R this is how we fetch new messages for now
                            let messages = client
                                .get_recent_history(&config.room)
                                .unwrap()
                                .items;

                            render_text(&compute_text(&messages), &mut stdout);
                        },
                        127 => {
                            // Backspace
                            current_message.pop();
                            write!(stdout, "{} {}",
                                   termion::cursor::Left(1),
                                   termion::cursor::Left(1)
                            ).unwrap();
                            stdout.flush().unwrap();
                        },
                        _ => {
                            current_message.push(c as char);
                            write!(stdout, "{}", c as char).unwrap();
                            //write!(stdout, "{} ", c).unwrap();
                            stdout.flush().unwrap();
                        },
                    }
                },
                Err(error) => continue,
            },
        }
    }

    stdout.flush().unwrap();
}
