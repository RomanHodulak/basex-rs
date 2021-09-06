mod common;

use basex;
use basex::{Client, ClientError};
use common::Asset;

#[test]
fn test_query_combines_2_documents() -> Result<(), ClientError> {
    let mut client = Client::connect("localhost", 1984, "admin", "admin")?;

    let info = client.create("lambada")?.without_input()?;
    assert!(info.starts_with("Database 'lambada' created"));

    let test_xml = Asset::get("sleeping.xml").unwrap();
    let info = client.add("sleeping", &mut test_xml.as_ref())?;
    assert!(info.starts_with("Resource(s) added"));

    let test_xml = Asset::get("powder.xml").unwrap();
    let info = client.add("powder", &mut test_xml.as_ref())?;
    assert!(info.starts_with("Resource(s) added"));

    let xquery = Asset::get("harvester.xq").unwrap();
    let mut query = client.query(&mut xquery.as_ref())?;
    let actual_result = query.execute()?;

    let expected_result = Asset::get("harvester_output.xml").unwrap();
    assert_eq!(actual_result.as_bytes(), expected_result.as_ref());

    let _ = query.close()?;
    Ok(())
}
