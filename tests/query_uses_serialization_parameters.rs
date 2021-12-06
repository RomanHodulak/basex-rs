use std::str::FromStr;
use basex;
use basex::{Client, ClientError, Options};

#[test]
fn test_query_uses_serialization_parameters() -> Result<(), ClientError> {
    let mut client = Client::connect("localhost", 1984, "admin", "admin")?;

    let database_name = "71e8a5a";
    let info = client.create(database_name)?
        .with_input("<None><Text/><Lala/><Papa/></None>")?;

    assert!(info.starts_with(&format!("Database '{}' created", database_name)));

    let expected_parameters = "encoding=US-ASCII,indent=no";
    let client = Options::from_str(expected_parameters)
        .unwrap()
        .save(client)?;

    let mut query = client.query("count(/None/*)")?;
    let actual_parameters = query.options()?;
    query.close()?;

    assert_eq!(expected_parameters, actual_parameters.to_string());
    Ok(())
}

#[test]
fn test_query_has_no_serialization_parameters_by_default() -> Result<(), ClientError> {
    let mut client = Client::connect("localhost", 1984, "admin", "admin")?;

    let database_name = "f5fef68";
    let info = client.create(database_name)?
        .with_input("<None><Text/><Lala/><Papa/></None>")?;

    assert!(info.starts_with(&format!("Database '{}' created", database_name)));

    let expected_parameters = "";
    let mut query = client.query("count(/None/*)")?;
    let actual_parameters = query.options()?;
    query.close()?;

    assert_eq!(expected_parameters, actual_parameters.to_string());
    Ok(())
}
