extern crate futures;
extern crate hyper;
extern crate serde_json;

// https://github.com/hyperium/hyper/blob/master/examples/web_api.rs
use futures::{future, Future, Stream}; // Stream is being used for concat2() ...
use hyper::{Body, Method, Request, Response, Server, StatusCode, header};
use hyper::service::service_fn;

use mongodb::coll::Collection;
use mongodb::{Bson, Client, ThreadedClient}; // ThreadClient needed for con.db()
use mongodb::db::ThreadedDatabase; // ThreadedDatabase needed for .connection()

enum DServerStatus {
    OK,
    STARTING,
    STOPPED
}

pub struct DServer {
    status: DServerStatus,
    mongo_port: u16,
    port: u16,
    network_db: Collection,
}

static NOTFOUND: &[u8] = b"Not Found";
static INDEX: &[u8] = b"<a href=\"test.html\">test.html</a>";

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type ResponseFuture = Box<Future<Item=Response<Body>, Error=GenericError> + Send>;

fn api_post_response(req: Request<Body>) -> ResponseFuture {
    // A web api to run against
    Box::new(req.into_body()
        .concat2() // Concatenate all chunks in the body
        .from_err()
        .and_then(|entire_body| {
            // TODO: Replace all unwraps with proper error handling
            let str = String::from_utf8(entire_body.to_vec())?;
            let mut data : serde_json::Value = serde_json::from_str(&str)?;
            data["test"] = serde_json::Value::from("test_value");
            let json = serde_json::to_string(&data)?;
            let response = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json))?;
            Ok(response)
        })
    )
}

fn api_get_response() -> ResponseFuture {
    let data = vec!["foo", "bar"];
    let res = match serde_json::to_string(&data) {
        Ok(json) => {
            Response::builder()
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json))
                .unwrap()
        }
        Err(_) => {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Internal Server Error"))
                .unwrap()
        }
    };

    Box::new(future::ok(res))
}

fn api_get_ip() -> ResponseFuture {
    Box::new(
        future::ok(Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("Internal Server Error"))
            .unwrap())
    )
}

fn response_examples(req: Request<Body>) -> ResponseFuture {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") | (&Method::GET, "/index.html") => {
            let body = Body::from(INDEX);
            Box::new(future::ok(Response::new(body)))
        },
        (&Method::POST, "/json_api") => {
            api_post_response(req)
        },
        (&Method::GET, "/json_api") => {
            api_get_response()
        },
        (&Method::GET, "/ip") => {
            api_get_ip()
        }
        _ => {
            // Return 404 not found response.
            let body = Body::from(NOTFOUND);
            Box::new(future::ok(Response::builder()
                                         .status(StatusCode::NOT_FOUND)
                                         .body(body)
                                         .unwrap()))
        }
    }
}

impl DServer {
    pub fn new(mongo_port: u16, port: u16, dbcon: Client) -> DServer {
        println!("Server create");
        
        let coll = dbcon.db("dvu_chain").collection("network");

        DServer { status: DServerStatus::STOPPED, mongo_port, port, network_db: coll}
    }
    
    pub fn start(&self) -> () {
        // This is our socket address...
        let addr = ([127, 0, 0, 1], self.port).into();

        // A `Service` is needed for every connection, so this
        // creates one from our `hello_world` function.


        // let new_service = move |&self| {
        //     service_fn( move |req, &self| {
        //         self.response_examples(req)
        //     })
        // };

        let new_service = move || {
            service_fn( move |req| {
                response_examples(req)
            })
        };

        let server = Server::bind(&addr)
            .serve(new_service)
            .map_err(|e| eprintln!("server error: {}", e));

        // Run this server for... forever!
        println!("Listening on http://{}", addr);
        hyper::rt::run(server);
    }
}