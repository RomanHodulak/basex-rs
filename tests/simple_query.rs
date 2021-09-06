use basex;
use basex::{Client, ClientError};

#[test]
fn test_executing_simple_query() -> Result<(), ClientError> {
    let mut client = Client::connect("localhost", 1984, "admin", "admin")?;

    let info = client.create("lambada")?
        .with_input(&mut "<None><Text></Text><Lala></Lala><Papa></Papa></None>".as_bytes())?;
    assert!(info.starts_with("Database 'lambada' created"));

    let mut query = client.query(&mut "count(/None/*)".as_bytes())?;
    let result = query.execute()?;
    assert_eq!(result, "3");

    let _ = query.close()?;
    Ok(())
}
