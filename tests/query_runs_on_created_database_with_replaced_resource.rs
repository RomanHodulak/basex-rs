mod common;

use basex;
use basex::{Client, ClientError};
use common::Asset;
use std::io::Read;

#[test]
fn test_query_runs_on_created_database_with_replaced_resource() -> Result<(), ClientError> {
    let mut client = Client::connect("localhost", 1984, "admin", "admin")?;

    let info = client.create("0076e54")?.without_input()?;

    assert!(info.starts_with("Database '0076e54' created"), "Actual info: {}", info);

    let test_xml = Asset::get("powder.xml").unwrap();
    let info = client.add("sleeping", &mut test_xml.data.as_ref())?;
    assert!(info.starts_with("Resource(s) added"), "Actual info: {}", info);

    let test_xml = Asset::get("sleeping.xml").unwrap();
    let info = client.replace("sleeping", &mut test_xml.data.as_ref())?;
    assert!(info.starts_with("1 resource(s) replaced"), "Actual info: {}", info);

    let query = client.query("count(//artikl)")?.without_info()?;
    let mut result = String::new();
    let mut response = query.execute()?;
    response.read_to_string(&mut result)?;
    assert_eq!(result, "3");

    let query = response.close()?;
    query.close()?;
    Ok(())
}
