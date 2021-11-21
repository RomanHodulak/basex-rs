mod common;

use basex;
use basex::{Client, ClientError};
use std::io::Read;

#[test]
fn test_command_after_unread_query_succeeds() -> Result<(), ClientError> {
    let mut client = Client::connect("localhost", 1984, "admin", "admin")?;

    let info = client.create("dda5457")?
        .with_input("<None><Text></Text><Lala></Lala><Papa></Papa></None>")?;
    assert!(info.starts_with("Database 'dda5457' created"));

    let query = client.query("count(/None/*)")?;
    let response = query.execute()?;
    let query = response.close()?;
    let mut client = query.close()?;

    let info = client.add("kakada", "<test></test>")?;
    assert!(info.starts_with("Resource(s) added"), "actual: {}", info);

    let query = client.query("count(/None/*)")?;

    let mut result = String::new();
    let mut response = query.execute()?;
    response.read_to_string(&mut result)?;
    assert_eq!(result, "3");

    let query = response.close()?;
    query.close()?;
    Ok(())
}
