use std::io::{empty, Read};
use basex;
use basex::{Client, ClientError};

#[test]
fn test_query_binds_context() -> Result<(), ClientError> {
    let mut client = Client::connect("localhost", 1984, "admin", "admin")?;

    let database_name = "27d2b96";
    let info = client.create(database_name)?.without_input()?;

    assert!(info.starts_with(&format!("Database '{}' created", database_name)));

    let mut response = {
        let mut query = client.query(&mut "/".as_bytes())?;
        query.context(&mut "<prdel></prdel>".as_bytes())?;
        query.execute()?
    };
    let mut actual_result = String::new();
    response.read_to_string(&mut actual_result)?;
    response.close().unwrap().close().unwrap();

    let expected_result = "<prdel/>";
    assert_eq!(expected_result, actual_result);
    Ok(())
}

#[test]
fn test_query_binds_empty_context() -> Result<(), ClientError> {
    let mut client = Client::connect("localhost", 1984, "admin", "admin")?;

    let database_name = "40fa157";
    let info = client.create(database_name)?.without_input()?;

    assert!(info.starts_with(&format!("Database '{}' created", database_name)));

    let mut response = {
        let mut query = client.query(&mut "/".as_bytes())?;
        query.context(&mut empty())?;
        query.execute()?
    };
    let mut actual_result = String::new();
    response.read_to_string(&mut actual_result)?;
    response.close().unwrap().close().unwrap();

    let expected_result = "";
    assert_eq!(expected_result, actual_result);
    Ok(())
}
