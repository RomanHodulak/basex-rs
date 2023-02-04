use basex::asynchronous::{Client, ConnectionError, QueryResponse, WithoutInfo};
use rocket::http::Status;
use rocket::request::Request;
use rocket::response::{self, Responder, Response};
use rocket::tokio::net::TcpStream;
use std::io::Cursor;

#[macro_use]
extern crate rocket;

struct QueryResponder(QueryResponse<TcpStream, WithoutInfo>);

impl From<QueryResponse<TcpStream, WithoutInfo>> for QueryResponder {
    fn from(value: QueryResponse<TcpStream, WithoutInfo>) -> Self {
        Self(value)
    }
}

impl<'r> Responder<'r, 'r> for QueryResponder {
    fn respond_to(self, _: &Request) -> response::Result<'r> {
        Response::build().streamed_body(self.0).ok()
    }
}

struct ErrorResponder(ConnectionError);

impl From<ConnectionError> for ErrorResponder {
    fn from(value: ConnectionError) -> Self {
        Self(value)
    }
}

impl<'r> Responder<'r, 'r> for ErrorResponder {
    fn respond_to(self, _: &Request) -> response::Result<'r> {
        let status = match self.0 {
            ConnectionError::Auth => Status::Unauthorized,
            ConnectionError::Io(..) => Status::InternalServerError,
            _ => Status::BadRequest,
        };
        let error = Cursor::new(self.0.to_string().as_bytes().to_vec());

        Response::build().status(status).streamed_body(error).ok()
    }
}

#[get("/<num>")]
async fn circle(num: u16) -> Result<QueryResponder, ErrorResponder> {
    let data = Cursor::new(include_str!("query.xq").as_bytes().to_vec());
    let client = Client::connect("localhost", 1984, "admin", "admin").await?;
    let mut query = client.query(data)?.without_info().await?;
    query.bind("points").await?.with_value(&num).await?;

    query.execute().await.map(Into::into).map_err(Into::into)
}

#[rocket::main]
async fn main() {
    let _ = rocket::build()
        .mount("/circle", routes![circle])
        .launch()
        .await
        .unwrap();
}
