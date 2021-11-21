mod common;

use basex;
use basex::{Client, ClientError};
use common::Asset;
use std::io::Read;

#[test]
fn test_query_runs_on_created_database_with_added_resources() -> Result<(), ClientError> {
    let mut client = Client::connect("localhost", 1984, "admin", "admin")?;

    let info = client.create("b34db74")?.without_input()?;

    assert!(info.starts_with("Database 'b34db74' created"));

    let test_xml = Asset::get("sleeping.xml").unwrap();
    let info = client.add("sleeping", &mut test_xml.as_ref())?;
    assert!(info.starts_with("Resource(s) added"));

    let test_xml = Asset::get("powder.xml").unwrap();
    let info = client.add("powder", &mut test_xml.as_ref())?;
    assert!(info.starts_with("Resource(s) added"));

    let query = client.query("count(//artikl)")?;
    let mut result = String::new();
    let mut response = query.execute()?;
    response.read_to_string(&mut result)?;
    assert_eq!(result, "3");

    let query = response.close()?;
    query.close()?;
    Ok(())
}
