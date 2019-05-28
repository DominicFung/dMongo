extern crate mongodb;

use std::net::Ipv4Addr;
use mongodb::{Client, ThreadedClient};
use sha2::{Sha256, Digest};

mod d_server;
mod d_client;

enum NodeStatus {
    Updating, // 
    Viewer,
    Delegate,
    Voting,
    Speaker
}

pub struct Node {
    ip: Ipv4Addr,
    port: u16,
    state: NodeStatus,
    client: d_client::DClient,
    server: d_server::DServer,
    mongo_port: u16
}

pub fn calculate_hash(input: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();

    hasher.input(input.clone().as_bytes());
    hasher.result()[..].to_vec()
}

impl Node {
    pub fn create(port: u16, mongo_port: u16 ) -> Node {

        let client = Client::connect("localhost", 27017)
            .expect("Failed to initialize client.");

        let client2 = client.clone();
        
        let addr = Ipv4Addr::new(127, 0, 0, 1);
        let client_handler = d_client::DClient::new(client);
        let server_handler = d_server::DServer::new(mongo_port, port, client2);

        let net_info = client_handler.get_network_info();


        Node { ip: addr, port, 
            state: NodeStatus::Viewer, client: client_handler,
            server: server_handler, mongo_port }
    }

    pub fn server_start(&self) -> () {
        self.server.start()
    }
}