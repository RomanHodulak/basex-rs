mod common;

use basex;
use basex::{Client, ClientError};

#[test]
fn test_query_with_undeclared_variable_fails() -> Result<(), ClientError> {
    let mut client = Client::connect("localhost", 1984, "admin", "admin")?;

    let mut query = client.query(&mut "$x".as_bytes())?;
    let actual_error = query.execute().unwrap_err();
    assert!(matches!(actual_error, ClientError::CommandFailed { message } if message == "" ));

    let _ = query.close()?;
    Ok(())
}
