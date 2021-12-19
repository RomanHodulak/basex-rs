mod common;

use basex::{Client, ClientError};

#[test]
fn test_command_with_invalid_argument_fails() -> Result<(), ClientError> {
    let client = Client::connect("localhost", 1984, "admin", "admin")?;
    let response = client.execute("OPEN dfasds")?;

    let actual_error = response.close().err().unwrap();
    assert!(matches!(
        actual_error,
        ClientError::CommandFailed(message) if message == "Database 'dfasds' was not found."
    ));

    Ok(())
}
