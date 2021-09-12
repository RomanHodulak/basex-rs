mod common;

use basex::{Client, ClientError};
use std::io::Read;

#[test]
fn test_command_with_empty_response_finishes() -> Result<(), ClientError> {
    let client = Client::connect("localhost", 1984, "admin", "admin")?;
    let mut response = client.execute("CLOSE")?;

    let mut buf: Vec<u8> = vec![];
    let size = response.read(&mut buf)?;
    let (_, info) = response.close()?;
    assert_eq!(0, size);
    assert_eq!("", info);

    Ok(())
}
