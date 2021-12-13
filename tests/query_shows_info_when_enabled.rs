use basex;
use basex::{Client, ClientError};

#[test]
fn test_query_shows_info_when_enabled() -> Result<(), ClientError> {
    let mut client = Client::connect("localhost", 1984, "admin", "admin")?;

    let database_name = "d601a46";
    let info = client.create(database_name)?
        .with_input("<None><Text/><Lala/><Papa/></None>")?;

    assert!(info.starts_with(&format!("Database '{}' created", database_name)));

    let (client, _) = client.execute("SET QUERYINFO true")?
        .close()?;

    let query = client.query("count(/None/*)")?;
    let mut query = query.execute()?.close()?;
    let actual_info = query.info()?;
    query.close()?;

    println!("{}", actual_info);
    Ok(())
}


#[test]
fn test_query_hides_info_by_default() -> Result<(), ClientError> {
    let mut client = Client::connect("localhost", 1984, "admin", "admin")?;

    let database_name = "7d28812";
    let info = client.create(database_name)?
        .with_input("<None><Text/><Lala/><Papa/></None>")?;

    assert!(info.starts_with(&format!("Database '{}' created", database_name)));

    let query = client.query("count(/None/*)")?;
    let mut query = query.execute()?.close()?;
    let actual_info = query.info()?;
    query.close()?;

    println!("{}", actual_info);
    Ok(())
}
