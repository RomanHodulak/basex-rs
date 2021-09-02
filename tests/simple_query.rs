use basex;
use basex::Client;

#[test]
fn test_executing_simple_query() {
    let mut client = Client::connect("localhost", 1984, "admin", "admin").unwrap();

    let info = client.create("lambada").unwrap()
        .with_input(&mut "<None><Text></Text><Lala></Lala><Papa></Papa></None>".as_bytes()).unwrap();
    assert!(info.starts_with("Database 'lambada' created"));

    let mut query = client.query(&mut "count(/None/*)".as_bytes()).unwrap();
    let result = query.execute().unwrap();
    assert_eq!(result, "3");

    let _ = query.close().unwrap();
}
