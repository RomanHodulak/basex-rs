mod common;

use basex;
use basex::{Client, ClientError};
use common::Asset;
use std::io::Read;

#[test]
fn test_query_creates_xml_on_created_database_with_added_resources() -> Result<(), ClientError> {
    let mut client = Client::connect("localhost", 1984, "admin", "admin")?;

    let info = client.create("1cb80e7")?.without_input()?;
    assert!(info.starts_with("Database '1cb80e7' created"));

    let test_xml = Asset::get("sleeping.xml").unwrap();
    let info = client.add("sleeping", &mut test_xml.as_ref())?;
    assert!(info.starts_with("Resource(s) added"));

    let test_xml = Asset::get("powder.xml").unwrap();
    let info = client.add("powder", &mut test_xml.as_ref())?;
    assert!(info.starts_with("Resource(s) added"));

    let xquery = Asset::get("harvester.xq").unwrap();
    let query = client.query(&mut xquery.as_ref())?;
    let mut actual_result = String::new();
    let mut response = query.execute()?;
    response.read_to_string(&mut actual_result)?;

    let expected_result = Asset::get("harvester_output.xml").unwrap();
    assert_eq!(actual_result.as_bytes(), expected_result.as_ref());

    let query = response.close()?;
    query.close()?;
    Ok(())
}
