mod common;

use basex;
use basex::{Client, ClientError};
use common::Asset;
use std::io::Read;

#[test]
fn test_executing_query_with_2_files() -> Result<(), ClientError> {
    let mut client = Client::connect("localhost", 1984, "admin", "admin")?;

    let info = client.create("lambada")?.without_input()?;

    assert!(info.starts_with("Database 'lambada' created"));

    let test_xml = Asset::get("sleeping.xml").unwrap();
    let info = client.add("sleeping", &mut test_xml.as_ref())?;
    assert!(info.starts_with("Resource(s) added"));

    let test_xml = Asset::get("powder.xml").unwrap();
    let info = client.add("powder", &mut test_xml.as_ref())?;
    assert!(info.starts_with("Resource(s) added"));

    let query = client.query(&mut "count(//artikl)".as_bytes())?;
    let mut result = String::new();
    let mut response = query.execute()?;
    response.read_to_string(&mut result)?;
    assert_eq!(result, "3");

    let mut query = response.close()?;
    query.close()?;
    Ok(())
}
