mod common;

use basex;
use basex::{Client, ClientError};

#[test]
fn test_command_without_open_database_fails() -> Result<(), ClientError> {
    let mut client = Client::connect("localhost", 1984, "admin", "admin")?;

    let actual_error = client.add("lambada", &mut "<test></test>".as_bytes()).unwrap_err();

    assert!(matches!(actual_error, ClientError::CommandFailed { message } if message == "No database opened."));

    Ok(())
}
