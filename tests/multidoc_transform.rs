mod common;

use basex;
use basex::Client;
use common::Asset;

#[test]
fn test_query_combines_2_documents() {
    let mut client = Client::connect("localhost", 1984, "admin", "admin").unwrap();

    let info = client.create("lambada").unwrap().without_input().unwrap();
    assert!(info.starts_with("Database 'lambada' created"));

    let test_xml = Asset::get("sleeping.xml").unwrap();
    let info = client.add("sleeping", test_xml.as_ref()).unwrap();
    assert!(info.starts_with("Resource(s) added"));

    let test_xml = Asset::get("powder.xml").unwrap();
    let info = client.add("powder", test_xml.as_ref()).unwrap();
    assert!(info.starts_with("Resource(s) added"));

    let xquery = Asset::get("harvester.xq").unwrap();
    let mut query = client.query(xquery.as_ref()).unwrap();
    let actual_result = query.execute().unwrap();

    let expected_result = Asset::get("harvester_output.xml").unwrap();
    assert_eq!(actual_result.as_bytes(), expected_result.as_ref());

    let _ = query.close().unwrap();
}
