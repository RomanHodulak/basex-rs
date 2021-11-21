use basex;
use basex::{Client, ClientError};
use std::io::Read;

#[test]
fn test_query_runs_on_created_database() -> Result<(), ClientError> {
    let mut client = Client::connect("localhost", 1984, "admin", "admin")?;

    let info = client.create("918f6e1")?
        .with_input("<None><Text></Text><Lala></Lala><Papa></Papa></None>")?;
    assert!(info.starts_with("Database '918f6e1' created"));

    let query = client.query("count(/None/*)")?;
    let mut result = String::new();
    let mut response = query.execute()?;
    response.read_to_string(&mut result)?;
    assert_eq!(result, "3");

    let query = response.close()?;
    query.close()?;
    Ok(())
}
