use std::io::Read;
use basex;
use basex::{Client, ClientError};

#[test]
fn test_query_binds_context() -> Result<(), ClientError> {
    let mut client = Client::connect("localhost", 1984, "admin", "admin")?;

    let database_name = "27d2b96";
    let info = client.create(database_name)?
        .with_input("<outer><one/><two/><three/></outer>")?;

    assert!(info.starts_with(&format!("Database '{}' created", database_name)));

    let mut response = {
        let mut query = client.query("/")?.without_info()?;
        query.context("<prdel></prdel>")?;
        query.execute()?
    };
    let mut actual_result = String::new();
    response.read_to_string(&mut actual_result)?;
    response.close()?.close()?;

    let expected_result = "<prdel/>";
    assert_eq!(expected_result, actual_result);
    Ok(())
}
