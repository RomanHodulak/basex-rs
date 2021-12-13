use basex;
use basex::{Client, ClientError};
use basex::analysis::Info;

#[test]
fn test_query_shows_info_when_enabled() -> Result<(), ClientError> {
    let mut client = Client::connect("localhost", 1984, "admin", "admin")?;

    let database_name = "d601a46";
    let info = client.create(database_name)?
        .with_input("<None><Text/><Lala/><Papa/></None>")?;

    assert!(info.starts_with(&format!("Database '{}' created", database_name)));

    let xquery = "count(/None/*)";
    let query = client.query(xquery)?.with_info()?;
    let mut query = query.execute()?.close()?;
    let actual_info = query.info()?;
    query.close()?;

    println!("{}", actual_info);
    assert_eq!(Some(database_name), actual_info.read_locking().as_ref().map(|v| v.as_str()));
    assert_eq!(None, actual_info.write_locking());
    assert_eq!(xquery, actual_info.query());
    Ok(())
}
