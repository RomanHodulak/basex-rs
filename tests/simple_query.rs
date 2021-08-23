use basex_client;
use basex_client::Client;

#[test]
fn test_executing_simple_query() {
    let mut client = Client::connect("localhost", 1984, "admin", "admin").unwrap();

    let info = client.create("lambada", Some("<None><Text></Text><Lala></Lala><Papa></Papa></None>")).unwrap();
    assert!(info.starts_with("Database 'lambada' created"));

    let mut query = client.query("count(/None/*)").unwrap();
    let result = query.execute().unwrap();
    assert_eq!(result, "3");

    let _ = query.close().unwrap();
}
