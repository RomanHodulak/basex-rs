#![feature(decl_macro)]

use std::fs::File;
use std::io::Seek;
use std::net::TcpStream;
use std::sync::RwLock;
use basex::Client;
use rocket::State;
use tempfile::tempfile;

#[macro_use] extern crate rocket_sync as rocket;

#[get("/<num>")]
fn circle(client: State<RwLock<Client<TcpStream>>>, num: u16) -> File {
    let client = client.write().unwrap().clone();
    let mut query = client.query("declare variable $points external;
    <polygon>
      {
        for $i in 1 to $points
        let $angle := 2 * math:pi() * number($i div $points)
        return <point x=\"{round(math:cos($angle), 8)}\" y=\"{round(math:sin($angle), 8)}\"></point>
      }
    </polygon>").unwrap().without_info().unwrap();
    query.bind("points").unwrap().with_value(num).unwrap();

    let mut response = query.execute().unwrap();
    let mut file = tempfile().unwrap();
    std::io::copy(&mut response, &mut file).unwrap();
    file.rewind().unwrap();

    file
}

fn main() {
    let client = Client::connect("localhost", 1984, "admin", "admin").unwrap();
    let client = RwLock::new(client);

    rocket::ignite()
        .manage(client)
        .mount("/circle", routes![circle])
        .launch();
}
