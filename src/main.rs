extern crate chrono;
extern crate hipchat_client;
extern crate serde;
extern crate toml;

#[macro_use]
extern crate serde_derive;

use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, Write};
use std::path::Path;

use chrono::prelude::*;
use hipchat_client::Client as HipchatClient;
use hipchat_client::message::Message;

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

fn print_message(message: Message) -> () {
    let parsed_date = DateTime::parse_from_rfc3339(&message.date);
    let from: String = message.from.map(|x| x.name).unwrap_or("Unknown".to_string());
    println!("[{}] [{}]: {}",
             parsed_date.unwrap().with_timezone(&Local).format("%m/%d %H:%M").to_string(),
             from,
             message.message);
}

fn print_messages(messages: Vec<Message>) -> () {
    // TODO: find a better way than .count() to consume an iter
    messages.into_iter().map(|m| print_message(m)).count();
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

fn main() {
    let config_path = get_config_path(env::args().nth(1));
    let config = load_config(config_path);
    let client = setup_client(&config);

    let messages = client
        .get_recent_history(&config.room)
        .unwrap()
        .items;

    println!("Messages for room {}", config.room);
    print_messages(messages);

    let message = prompt_for_message();
    client.send_message(&config.room, message);
}
