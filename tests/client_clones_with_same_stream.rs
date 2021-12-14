mod common;

use basex;
use basex::{Client, ClientError};
use std::io::Read;

#[test]
fn test_client_clones_with_same_stream() -> Result<(), ClientError> {
    let client_foo = Client::connect("localhost", 1984, "admin", "admin")?;
    let (client_foo, _) = client_foo.execute("SET QUERYINFO true")?.close()?;
    let client_bar = client_foo.clone();
    client_bar.execute("SET QUERYINFO false")?.close()?;

    let mut result = String::new();
    let mut response = client_foo.execute("GET QUERYINFO")?;
    response.read_to_string(&mut result).unwrap();
    response.close()?;

    assert_eq!("QUERYINFO: false\n", result);
    Ok(())
}
