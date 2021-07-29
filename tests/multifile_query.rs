use basex_client;
use basex_client::Client;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "tests/files/"]
#[prefix = ""]
struct Asset;

#[test]
fn test_executing_query_with_2_files() {
    let mut client = Client::connect("basex", 1984, "admin", "admin").unwrap();

    let info = client.create("lambada2", None).unwrap();
    assert!(info.starts_with("Database 'lambada2' created"));

    let test_xml = Asset::get("sleeping.xml").unwrap();
    let test_xml = std::str::from_utf8(test_xml.as_ref()).unwrap();
    let info = client.add("sleeping", Some(test_xml)).unwrap();
    assert!(info.starts_with("Resource(s) added"));

    let test_xml = Asset::get("powder.xml").unwrap();
    let test_xml = std::str::from_utf8(test_xml.as_ref()).unwrap();
    let info = client.add("powder", Some(test_xml)).unwrap();
    assert!(info.starts_with("Resource(s) added"));

    let mut query = client.query("count(//artikl)").unwrap();
    let result = query.execute().unwrap();
    assert_eq!(result, "3");

    let _ = query.close().unwrap();
}
