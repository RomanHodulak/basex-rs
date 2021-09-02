mod common;

use basex;
use basex::Client;
use common::Asset;
use std::io::Read;

#[test]
fn test_executing_query_with_2_files() {
    let mut client = Client::connect("localhost", 1984, "admin", "admin").unwrap();

    let mut info = client.create("lambada").unwrap().without_input().unwrap();

    assert!(info.starts_with("Database 'lambada' created"));

    let test_xml = Asset::get("sleeping.xml").unwrap();
    let info = client.add("sleeping", test_xml.as_ref()).unwrap();
    assert!(info.starts_with("Resource(s) added"));

    let test_xml = Asset::get("powder.xml").unwrap();
    let info = client.add("powder", test_xml.as_ref()).unwrap();
    assert!(info.starts_with("Resource(s) added"));

    let mut query = client.query("count(//artikl)".as_bytes()).unwrap();
    let result = query.execute().unwrap();
    assert_eq!(result, "3");

    let _ = query.close().unwrap();
}
