#[macro_use]
extern crate clap;
use clap::App;

extern crate mongodb;
use mongodb::{Bson, bson, doc};
use mongodb::{Client, ThreadedClient};
use mongodb::db::ThreadedDatabase;

use std::str;
use std::process::Command;

// connection pools not working according to example: https://github.com/mongodb-labs/mongo-rust-driver-prototype/issues/139
//use std::sync::Arc;
//use mongo_driver::client::{ClientPool,Uri};

mod account;
mod node;

fn main() {
    println!("Welcome to dMongo");

    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches(); // throws an error if there is no input file

    let db_port = matches.value_of("dbport").unwrap().parse::<u16>().unwrap();
    let port = matches.value_of("port").unwrap().parse::<u16>().unwrap();
    let db_path = matches.value_of("dbpath").unwrap();

    println!("Using db_path: {}", db_path);
    println!("Using db_port: {}", db_port);
    println!("Using port:    {}", port);

    // You can handle information about subcommands by requesting their matches by name
    // (as below), requesting just the name used, or both at the same time
    if let Some(matches) = matches.subcommand_matches("test") {
        if matches.is_present("debug") {
            println!("Printing debug info...");
        } else {
            println!("Printing normally...");
        }
    }

    let output = if cfg!(target_os = "windows") {
        println!("This is Windows");
        // Command::new("start")
        //     .args(&["D:\\dMongo\\bin\\start-mongo.bat"])
        //     .output()
        //     .expect("failed to execute process")
        Command::new("cmd")
            .args(&["/C",".\\bin\\start-mongo.bat", &db_path, &db_port.to_string()])
            .output()
            .expect("failed to execute process")
        // Command::new("cmd")
        //     .args(&["/C", "echo","%cd%"])
        //     .output()
        //     .expect("failed to execute process")
    } else {
        Command::new("sh")
            .args(&["./bin/start-mongo.sh", &db_path, &db_port.to_string()])
            .output()
            .expect("failed to execute process")
    };

    //let hello = output.stdout;
    let hello = str::from_utf8(&output.stdout).unwrap();
    println!("{}", hello);

    // more program logic goes here...
    // MONGO TEST
    let client = Client::connect("localhost", db_port)
    .expect("Failed to initialize client.");

    // cloning is said to be a good idea .... https://github.com/mongodb-labs/mongo-rust-driver-prototype/issues/139
    let client2 = client.clone();

    let my_account = account::Account::recreate("ea102d82-5458-4a21-a26e-5e8b42218180", "dom@email.com", "asdhgashjaj", client);
    my_account.get_uuid();

    // // TEST node instatiation
    let test_node = node::Node::create(port, db_port);
    test_node.server_start();
}
