mod common;

use basex;
use basex::{Client, ClientError};
use common::Asset;

#[test]
fn test_executing_query_on_db_with_replaced_resource() -> Result<(), ClientError> {
    let mut client = Client::connect("localhost", 1984, "admin", "admin")?;

    let info = client.create("lambada")?.without_input()?;

    assert!(info.starts_with("Database 'lambada' created"), "Actual info: {}", info);

    let test_xml = Asset::get("powder.xml").unwrap();
    let info = client.add("sleeping", &mut test_xml.as_ref())?;
    assert!(info.starts_with("Resource(s) added"), "Actual info: {}", info);

    let test_xml = Asset::get("sleeping.xml").unwrap();
    let info = client.replace("sleeping", &mut test_xml.as_ref())?;
    assert!(info.starts_with("1 resource(s) replaced"), "Actual info: {}", info);

    let mut query = client.query(&mut "count(//artikl)".as_bytes())?;
    let result = query.execute()?;
    assert_eq!(result, "3");

    let _ = query.close()?;
    Ok(())
}
