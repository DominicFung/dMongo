extern crate mongodb;
use mongodb::{bson, doc};
use mongodb::{Client, ThreadedClient};
use mongodb::db::ThreadedDatabase;
use mongodb::coll::Collection;
use uuid::Uuid;

#[derive(Debug)]
pub struct Account {
    uuid: String,
    email: String,
    activity_points: i64,
    encrypted_pass: String,
    blockchain: Collection
}

fn get_ap(uuid: &str, blockchain: &Collection) -> i64 {

    let coll = blockchain.clone();
    let mut total = 0;
    let doc = doc! {
        "uuid": uuid
    };

    // Find the document and receive a cursor
    let mut cursor = coll.find(Some(doc.clone()), None)
        .ok().expect("Failed to execute find.");

    while let item = cursor.next() {
        match item {
            Some(Ok(doc)) => match doc.get("amount") {
                Some(amount) => {
                    match amount.to_string().parse::<i64>() {
                        Ok(i) => total = total + i,
                        Err(e) => println!("error paring i: {:?}", e),
                    }
                }
                None => {
                    panic!("Expected some amount!")
                },
            },
            Some(Err(_)) => panic!("Failed to get next from server!"),
            None => break,
        }
    }

    println!("End While: {}", total);
    total
}

impl Account {

    #[allow(dead_code)]
    pub fn create(email: &str, encrypted_pass: &str, con: Client) -> Account {
        let uuid = Uuid::new_v4().to_string();
        let ref_email = email.to_string();
        let ref_encrypted_pass = encrypted_pass.to_string();

        let coll = con.db("dvu_chain").collection("chain");
        let activity_points: i64 = get_ap(&uuid, &coll);

        Account {uuid, email: ref_email, activity_points, encrypted_pass: ref_encrypted_pass, blockchain: coll}
    }

    // Refactor to polymorphism using Box<T> later ..
    pub fn recreate(uuid: &str , email: &str, encrypted_pass: &str, con: Client) -> Account {
        let ref_uuid = uuid.to_string();
        let ref_email = email.to_string();
        let ref_encrypted_pass = encrypted_pass.to_string();

        let coll = con.db("dvu_chain").collection("chain");
        let activity_points: i64 = get_ap(&uuid, &coll);

        Account {uuid: ref_uuid, email: ref_email, activity_points, encrypted_pass: ref_encrypted_pass, blockchain: coll}
    }

//    pub fn new(account: Box<>) -> Account {

//    }

    pub fn get_uuid(&self) -> &String {
        println!("{}", &self.uuid);
        &self.uuid
    }

    

    // pub fn calculate_activity();
}

