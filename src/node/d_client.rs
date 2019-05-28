extern crate reqwest;
extern crate mongodb;
extern crate rand;

use mongodb::coll::Collection;
use mongodb::{Bson, Client, ThreadedClient}; // ThreadClient needed for con.db()
use mongodb::db::ThreadedDatabase; // ThreadedDatabase needed for .connection()
use std::collections::HashSet;
use std::collections::HashMap;

//use rand::{thread_rng, Rng};

// 
//use rand::seq::IteratorRandom; //https://docs.rs/rand/0.6.5/rand/seq/trait.IteratorRandom.html
use rand::seq::SliceRandom; //https://docs.rs/rand/0.6.5/rand/seq/trait.SliceRandom.html#tymethod.choose_multiple

use crate::node;

enum DClientStatus {
    OK,
    STARTING,
    STOPPED
}

pub struct DClient {
    status: DClientStatus,
    network_db: Collection,
    network_list: HashSet<String>
}

// default network will be stored in mongo
pub fn get_stored_network(network: &Collection) -> HashSet<String> {
    let coll = network.clone();
    let mut net_list = HashSet::new();

    let cursor = coll.find(None, None)
        .ok().expect("Failed to execute find.");

    for result in cursor {
        if let Ok(item) = result {
            let mut ip_port = String::new();
            if let Some(&Bson::String(ref ip)) = item.get("ip") {
                ip_port.push_str(ip);
            } else {
                println!("ERROR: IP NOT FOUND IN DB");
            }

            ip_port.push_str(":");
            
            if let Some(&Bson::String(ref port)) = item.get("port") {
                ip_port.push_str(port);
            } else {
                println!("ERROR: PORT NOT FOUND IN DB");
            }

            net_list.insert(ip_port);
        }
    }

    net_list
}

pub fn get_net_from_single_source(ip: &str, port: &str) -> Result<String, &'static str> {

    let mut http_string = String::from("http://");
    http_string.push_str(ip);
    http_string.push(':');
    http_string.push_str(port);
    http_string.push_str("/ip");
    
    match reqwest::get(&http_string) {
        Ok(mut response) => {
            if response.status() == reqwest::StatusCode::OK {
                match response.text() {
                    Ok(text) => {
                        println!("Response Text: {}", text);
                        Ok(text)
                    },
                    Err(_) => Err("Could not read response text.")
                }
            } else {
                Err("Response was not 200 OK.")
            }
        }
        Err(_) => Err("Could not make request!")
    }
}

pub fn hashset_to_vec(hashset: &HashSet<String>) -> Vec<String> {

    let mut vec = Vec::new();
    
    for item in hashset {
        vec.push(item.clone());
    }

    vec

}

impl DClient {
    pub fn new(dbcon: Client) -> DClient {

        let coll = dbcon.db("dvu_chain").collection("network");
        let net_list = get_stored_network(&coll);
        
        DClient { status: DClientStatus::STOPPED, network_db: coll, network_list: net_list }
    }

    pub fn get_network_info(&self) -> Result<String, &'static str> {

        let net_length = self.network_list.len();
        let mut net_from_system = HashMap::new();

        let mut max_hash = Vec::new();
        let mut max_hash_text = String::new();
        let mut loop_total = 0;

        if net_length <= 4 {
            
            for ep in &self.network_list {
                println!("Set: {}", ep.to_string());

                let ip_ref: Vec<&str> = ep.split(":").collect();
                let ip = ip_ref[0];

                let port_ref: Vec<&str> = ep.split(":").collect();
                let port = port_ref[1];

                let mut net_data_hash = Vec::new();
                let mut net_data = String::new();
                match get_net_from_single_source(&ip, &port) {
                    Ok(data) => {
                        net_data_hash.extend(node::calculate_hash(&data));
                        net_data = data;
                    }
                    Err(_) => println!("nothing came back from a machine ..")
                }

                if !net_from_system.contains_key(&net_data_hash) {
                    // starting case
                    if loop_total == 0 {
                        max_hash = net_data_hash.clone();
                        max_hash_text = net_data.clone();
                    }

                    net_from_system.insert(
                        net_data_hash, 1
                    );
                } else {
                    match net_from_system.get(&net_data_hash) {
                        Some(val) => {

                            if net_from_system[&max_hash] == net_from_system[&net_data_hash] {
                                max_hash = net_data_hash.clone();
                                max_hash_text = net_data.clone();
                            }

                            net_from_system.insert(net_data_hash, val + 1); // refactor later to: https://doc.rust-lang.org/std/collections/struct.HashMap.html

                        }
                        None => panic!("There is no way this happens ..")
                    }
                }

                loop_total = loop_total + 1;
                
            }
        } else {

            // randomly select 4 for comparison
            let mut rng = &mut rand::thread_rng();
            //let ref_sample = &self.network_list;
            //let ref_sample = "Hello, audience!".as_bytes();
            let ref_sample = hashset_to_vec(&self.network_list);
            let sample: Vec<String> = ref_sample.choose_multiple(&mut rng, 4).cloned().collect();
            println!("{:?}", sample);

            for ep in &sample {

                println!("Set: {}", ep.to_string());

                let ip_ref: Vec<&str> = ep.split(":").collect();
                let ip = ip_ref[0];

                let port_ref: Vec<&str> = ep.split(":").collect();
                let port = port_ref[1];

                let mut net_data_hash = Vec::new();
                let mut net_data = String::new();
                match get_net_from_single_source(&ip, &port) {
                    Ok(data) => {
                        net_data_hash.extend(node::calculate_hash(&data));
                        net_data = data;
                    }
                    Err(_) => println!("nothing came back from a machine ..")
                }

                if !net_from_system.contains_key(&net_data_hash) {
                    // starting case
                    if loop_total == 0 {
                        max_hash = net_data_hash.clone();
                        max_hash_text = net_data.clone();
                    }

                    net_from_system.insert(
                        net_data_hash, 1
                    );
                } else {
                    match net_from_system.get(&net_data_hash) {
                        Some(val) => {

                            if net_from_system[&max_hash] == net_from_system[&net_data_hash] {
                                max_hash = net_data_hash.clone();
                                max_hash_text = net_data.clone();
                            }

                            net_from_system.insert(net_data_hash, val + 1); // refactor later to: https://doc.rust-lang.org/std/collections/struct.HashMap.html
                        }
                        None => panic!("There is no way this happens ..")
                    }
                }

                loop_total = loop_total + 1;
            }
        }

        match net_from_system.get(&max_hash) {
            Some(val) => {
                let d = *val as f64 / loop_total as f64;

                // IMPORTANT - this is our acceptance ratio
                if d > 0.666 {
                    Ok(max_hash_text)
                } else {
                    //let mut ret_err = String::from("recieved network data is incompatable, ratio: ");
                    //ret_err.push_str(&d.to_string());
                    println!("recieved network data is incompatable, ratio: {}", d);
                    Err("recieved network data is incompatable, ratio is not above 0.66")
                }
            }
            None => Err("unable to get max_hash from net_from_system")
        }
    }

    // REGISTER:
    //    POST request with the following details:
    //     - hash of DB
    //     - hash of blockchain
    //     - hash of network?
    fn register(&self) -> Result<bool, &'static str> {
        match reqwest::get("http://httpbin.org/ip") {
            Ok(mut response) => {
                if response.status() == reqwest::StatusCode::OK {
                    match response.text() {
                        Ok(text) => {
                            println!("Response Text: {}", text);
                            Ok(true)
                        },
                        Err(_) => Err("Could not read response text.")
                    }
                } else {
                    Err("Response was not 200 OK.")
                }
            }
            Err(_) => Err("Could not make request!")
        }
    }

    // fn get_network_hash($self) -> String {
    //     self.network_list.ha
    // }

    // fn get_network(&self) -> Result<bool, &'static str> {
    //     match reqwest::get("http://httpbin.org/ip") {
    //         Ok(mut response) => {
    //             if response.status() == reqwest::StatusCode::OK {
    //                 match response.text() {
    //                     Ok(text) => {
    //                         println!("Response Text: {}", text);
    //                         Ok(true)
    //                     },
    //                     Err(_) => Err("Could not read response text.")
    //                 }
    //             } else {
    //                 Err("Response was not 200 OK.")
    //             }
    //         }
    //         Err(_) => Err("Could not make request!")
    //     }
    // }

    // fn get_blocks(&self) -> Result<bool, &'static str> {

    // }

    // fn getdb_data(&self) -> Result<bool, &'static str> {

    // }
}