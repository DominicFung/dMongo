extern crate hyper;
extern crate futures;
extern crate reqwest;
extern crate serde_json;
extern crate mongodb;
extern crate regex;

use hyper::service::{NewService, Service};
use hyper::{Body, Error, Response, StatusCode, Request, Method, Server};
use futures::{future, Future};

use mongodb::coll::Collection;
use mongodb::coll::options::FindOptions;
use mongodb::{bson, doc};
use mongodb::{Bson, Client, ThreadedClient}; // ThreadClient needed for con.db()
use mongodb::db::ThreadedDatabase; // ThreadedDatabase needed for .connection()

use std::collections::HashSet;
use std::collections::HashMap;

// use serde_json::{Result, Value, Error};
use serde_json::Value;
// SENDING JSON BACK: https://stackoverflow.com/questions/48936061/how-to-fix-the-trait-stdconvertfromserde-jsonvalue-is-not-implemented

use regex::Regex;

#[derive(Copy, Clone)]
enum DServerStatus {
    OK,
    STARTING,
    STOPPED
}

pub struct DServer {
    status: DServerStatus,
    public_ip: String,
    port: u16,

    mongo_con: Option<Client>,
    mongo_port: u16,
    mongo_db: String,

    mongo_net: String,
    mongo_data: Vec<String>,
    mongo_block: String
}

impl NewService for DServer {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = Error;
    type InitError = Error;
    type Service = DServer;
    type Future = Box<Future<Item = Self::Service, Error = Self::InitError> + Send>;
    fn new_service(&self) -> Self::Future {
        Box::new(future::ok(Self {
            status: self.status.clone(),
            public_ip: self.public_ip.clone(),
            port: self.port,
            
            mongo_con: self.mongo_con.clone(),
            mongo_port: self.mongo_port,
            mongo_db: self.mongo_db.clone(),
            
            mongo_net: self.mongo_net.clone(),
            mongo_block: self.mongo_block.clone(),
            mongo_data: self.mongo_data.clone()
        }))
    }
}

impl Service for DServer {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = Error;
    type Future = Box<Future<Item = Response<Body>, Error = Error> + Send>;
    
    fn call(&mut self, req: Request<Self::ReqBody>) -> Self::Future {

        // OLD SYSTEM + NEW SYSTEM (regex_path)
        match (req.method(), req.uri().path()) {
            (&Method::GET, "/du-chain/api/ip")      => self.api_get_ip(),
            (&Method::GET, "/du-chain/api/network") => self.api_get_network(),
            (&Method::GET, "/du-chain/api/test") => self.api_test(),
            _ => self.regex_path(req.method(), req.uri().path()),
        }
    }
}

fn compare_pub_ip() -> Result<String, String> {
    let mut guess_ip = String::new();
    let mut error_message = String::new();

    match get_public_ip("https://api.ipify.org?format=json") {
        Ok(check) => guess_ip = check,
        Err(_) => {
            println!("get_public_ip('https://api.ipify.org?format=json' returned error)");
            error_message = "get_public_ip('https://api.ipify.org?format=json' returned error)".to_string()
        }
    }

    if !error_message.is_empty() { return Err(error_message) }

    match get_public_ip("https://ip.seeip.org/jsonip?"){
        Ok(check) => {
            if guess_ip != check {
                println!("IPs did not match, guess_ip: {}, check: {}", guess_ip, check);
            }
        },
        Err(_) => {
            println!("get_public_ip('https://ip.seeip.org/jsonip?' returned error)");
            error_message = "get_public_ip('https://ip.seeip.org/jsonip?' returned error)".to_string();
        }
    }

    if !error_message.is_empty() { return Err(error_message) }
    else {
        println!("IP MATCH: {}", guess_ip);
        return Ok(guess_ip) 
    }

}

fn get_public_ip(check: &str) -> Result<String, &'static str> {
    match reqwest::get(check) {
        Ok(mut response) => {
            if response.status() == reqwest::StatusCode::OK {
                match response.text() {
                    Ok(text) => {
                        println!("Response Text: {}", text);
                        let s: String = text.to_string();
                        let v: Value = serde_json::from_str(&s).unwrap();
                        println!("ip from json: {}", v["ip"]);
                        Ok(format!("{}", v["ip"]))
                    },
                    Err(_) => Err("Could not read response text.")
                }
            } else {
                Err("Response was not 200 OK.")
            }
        },
        Err(_) => Err("Could not make request!")
    }
}

pub fn get_stored_network(network: &Collection) -> HashSet<String> {
    println!("in get_stored_network() 1");
    let coll = network.clone();
    println!("in get_stored_network() 2");
    let mut net_list = HashSet::new();

    let cursor = coll.find(None, None)
        .ok().expect("Failed to execute find.");

    println!("in get_stored_network() 3");

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

impl DServer {

    pub fn new(mongo_port: u16, port: u16) -> DServer {
        println!("Server create");

        let pub_ip = compare_pub_ip().unwrap();
        
        let mut mongo_data = Vec::new();
        mongo_data.push("accounts".to_string());
        mongo_data.push("posts".to_string());

        DServer { status: DServerStatus::STOPPED, port, public_ip: pub_ip, 
            mongo_con : None, mongo_port, mongo_db: "dvu_chain".to_string(), 
            mongo_net: "network".to_string(),
            mongo_block: "chain".to_string(),
            mongo_data }
    }

    fn start(self) {
        let addr = format!("127.0.0.1:{}", self.port).parse().unwrap();
        let server = Server::bind(&addr).serve(self)
            .map_err(|e| eprintln!("error: {}", e));
        println!("Serving at {}", addr);

        hyper::rt::run(server);
    }

    fn get_collection(&mut self, coll: &str) -> Collection {
        // NEEDED - why?
        // scenario: mongo connection is started -> | server start | -> use mongo connection => error
        //           Thus, we need to start the connection only AFTER server start
        //           This helper function will detect whether or not there is already a connection, if not, we will start a new one

        match &self.mongo_con {
            Some(client) => client.db(&self.mongo_db).collection(coll),
            None => {
                let client = Client::connect("localhost", self.mongo_port)
                    .expect("Failed to initialize client.");

                self.mongo_con = Some(client.clone());
                client.db(&self.mongo_db).collection(coll)
            }
        }
    }

//         ========================
//        |    API PATH MATCHER    |
//         ========================
    fn regex_path(&mut self, method: &Method, path: &str) -> Box<Future<Item = Response<Body>, Error = Error> + Send> {

        println!("in regex_path(), PATH: {}",path);
    
        // PATH: /du-chain/api/chain/:start:-:end:
        let re = Regex::new(r"^/du-chain/api/chain/\d+-\d+$").unwrap();
        if re.is_match(path) { 
            println!("CALL: /du-chain/api/chain/:start:-:end:");

            // get start and end
            let path_clone = path.to_string().clone();
            let new_str = path_clone.split("/du-chain/api/chain/").collect::<Vec<_>>()[1];
            let start = new_str.clone().to_string().split("-").collect::<Vec<_>>()[0].parse::<u16>().unwrap();
            let end = new_str.to_string().split("-").collect::<Vec<_>>()[1].parse::<u16>().unwrap();

            return self.api_get_blocks(start, end);
        }

        // PATH: /du-chain/api/data/{collection name}
        let re = Regex::new(r"^/du-chain/api/data/[a-zA-Z_]+$").unwrap();
        if re.is_match(path) {
            println!("CALL: /du-chain/api/data/{{collection name}}");

            // get collection name
            let path_clone = path.to_string().clone();
            let col_name = path_clone.split("/du-chain/api/data/").collect::<Vec<_>>()[1];

            return self.api_get_db_data(col_name);
        }



        Box::new(future::ok(
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Path Not Found"))
                .unwrap()
            ))
    }

//         #####################
//        #         API         #
//         #####################
    fn api_get_ip(&mut self) -> Box<Future<Item = Response<Body>, Error = Error> + Send> {

        println!("in api_get_ip()");

        // let net_string = self.mongo_net.clone();
        let coll = self.get_collection(&self.mongo_net.clone());
        println!("in api_get_ip() 1");
        let net_list = get_stored_network(&coll);
        println!("in api_get_ip() 2");

        Box::new(futures::future::ok(
            Response::builder()
                .status(StatusCode::OK)
                .body(Body::from("TBA"))
                .unwrap(),
        ))
    }

    fn api_get_network(&mut self) -> Box<Future<Item = Response<Body>, Error = Error> + Send> {

        println!("api_get_network()");
        let coll = self.get_collection(&self.mongo_net.clone());
        let cursor = coll.find(None, None).ok().expect("Failed to execute mongodb find()");

        let docs: Vec<_> = cursor.map(|doc| doc.unwrap()).collect();
        let serialized = serde_json::to_string(&docs).unwrap();
        
        Box::new(futures::future::ok(
            Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(serialized))
                .unwrap(),
        ))
    }

    fn api_get_db_data(&mut self, col_name: &str) -> Box<Future<Item = Response<Body>, Error = Error> + Send> {

        println!("api_get_blocks()");
        let position = self.mongo_data.iter().cloned().position(| r: String | r == col_name);
        let mut index: i32 = -1;

        match position {
            Some(num) => {
                println!("Position in mongo_data: {}", num);
                index = num as i32;
            },
            None => println!("Position in mongo_data unfound."),
        }

        if index >= 0 {
            let coll = self.get_collection(col_name);
            let cursor = coll.find(None, None).ok().expect("Failed to execute mongodb find()");

            let docs: Vec<_> = cursor.map(|doc| doc.unwrap()).collect();
            let serialized = serde_json::to_string(&docs).unwrap();

            Box::new(futures::future::ok(
                Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::from(serialized))
                    .unwrap(),
            ))

        } else {
            Box::new(futures::future::ok(
                Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from("Path Not Found"))
                    .unwrap(),
            ))
        }
    }

    fn api_get_blocks(&mut self, start: u16, end: u16) -> Box<Future<Item = Response<Body>, Error = Error> + Send> {

        // use findOptions
        // https://docs.rs/mongodb/0.3.12/mongodb/coll/options/struct.FindOptions.html

        println!("api_get_blocks()");
        let coll = self.get_collection(&self.mongo_block.clone());

        let mut options = FindOptions::new();
        options.skip = Some(start as i64);
        if end > start {
            options.limit = Some((end - start) as i64)
        } else {
            return Box::new(futures::future::ok(
                Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::from("[]"))
                    .unwrap(),
            ));
        }

        let cursor = coll.find(None, Some(options)).ok().expect("Failed to execute mongodb find()");
        let docs: Vec<_> = cursor.map(|doc| doc.unwrap()).collect();
        let serialized = serde_json::to_string(&docs).unwrap();
        
        Box::new(futures::future::ok(
            Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(serialized))
                .unwrap(),
        ))
    }

    fn api_test(&mut self) -> Box<Future<Item = Response<Body>, Error = Error> + Send> {

        Box::new(futures::future::ok(
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Path Not Found"))
                .unwrap(),
        ))
    }
}