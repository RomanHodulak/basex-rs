mod common;

use basex;
use basex::{Client, ClientError};
use std::io::Read;

#[test]
fn test_command_with_open_query_succeeds() -> Result<(), ClientError> {
    let mut client = Client::connect("localhost", 1984, "admin", "admin")?;

    let query = client.query(&mut "count(/None/*)".as_bytes())?;

    let info = client.create("lambada")?
        .with_input(&mut "<None><Text></Text><Lala></Lala><Papa></Papa></None>".as_bytes())?;
    assert!(info.starts_with("Database 'lambada' created"));

    let info = client.add("kakada", &mut "<test></test>".as_bytes())?;
    assert!(info.starts_with("Resource(s) added"));

    let mut result = String::new();
    let mut response = query.execute()?;
    response.read_to_string(&mut result)?;
    assert_eq!(result, "3");

    let mut query = response.close()?;
    query.close()?;
    Ok(())
}
