mod common;

use basex_client;
use basex_client::Client;
use common::Asset;

#[test]
fn test_query_combines_2_documents() {
    let mut client = Client::connect("basex", 1984, "admin", "admin").unwrap();

    let info = client.create("lambada", None).unwrap();
    assert!(info.starts_with("Database 'lambada' created"));

    let test_xml = Asset::get("sleeping.xml").unwrap();
    let test_xml = std::str::from_utf8(test_xml.as_ref()).unwrap();
    let info = client.add("sleeping", Some(test_xml)).unwrap();
    assert!(info.starts_with("Resource(s) added"));

    let test_xml = Asset::get("powder.xml").unwrap();
    let test_xml = std::str::from_utf8(test_xml.as_ref()).unwrap();
    let info = client.add("powder", Some(test_xml)).unwrap();
    assert!(info.starts_with("Resource(s) added"));

    let xquery = Asset::get("harvester.xq").unwrap();
    let xquery = std::str::from_utf8(xquery.as_ref()).unwrap();
    let mut query = client.query(xquery).unwrap();
    let actual_result = query.execute().unwrap();

    let expected_result = Asset::get("harvester_output.xml").unwrap();
    let expected_result = std::str::from_utf8(expected_result.as_ref()).unwrap();
    assert_eq!(actual_result, expected_result);

    let _ = query.close().unwrap();
}
