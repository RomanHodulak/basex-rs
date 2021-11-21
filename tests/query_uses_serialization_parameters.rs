use basex;
use basex::{Client, ClientError};

#[test]
fn test_query_uses_serialization_parameters() -> Result<(), ClientError> {
    let mut client = Client::connect("localhost", 1984, "admin", "admin")?;

    let database_name = "test_query_uses_serialization_parameters";
    let info = client.create(database_name)?
        .with_input("<None><Text></Text><Lala></Lala><Papa></Papa></None>")?;

    assert!(info.starts_with(&format!("Database '{}' created", database_name)));

    let expected_parameters = "encoding=US-ASCII,indent=no";
    let (client, _) = client.execute(&format!("SET SERIALIZER {}", expected_parameters))?
        .close()?;

    let mut query = client.query("count(/None/*)")?;
    let actual_parameters = query.options()?;
    query.close()?;

    assert_eq!(expected_parameters, actual_parameters);
    Ok(())
}

#[test]
fn test_query_has_no_serialization_parameters_by_default() -> Result<(), ClientError> {
    let mut client = Client::connect("localhost", 1984, "admin", "admin")?;

    let database_name = "test_query_has_no_serialization_parameters_by_default";
    let info = client.create(database_name)?
        .with_input("<None><Text></Text><Lala></Lala><Papa></Papa></None>")?;

    assert!(info.starts_with(&format!("Database '{}' created", database_name)));

    let expected_parameters = "";
    let mut query = client.query("count(/None/*)")?;
    let actual_parameters = query.options()?;
    query.close()?;

    assert_eq!(expected_parameters, actual_parameters);
    Ok(())
}
