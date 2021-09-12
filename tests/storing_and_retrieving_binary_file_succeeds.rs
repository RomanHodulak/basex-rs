mod common;

use basex;
use basex::{Client, ClientError};
use std::io::Read;

#[test]
fn test_storing_and_retrieving_binary_file_succeeds() -> Result<(), ClientError> {
    let mut client = Client::connect("localhost", 1984, "admin", "admin")?;

    let info = client.create("lambada")?.without_input()?;
    assert!(info.starts_with("Database 'lambada' created"));

    let expected_result = [6u8, 1, 0xFF, 3, 4, 0u8, 6, 5];
    client.store("blob", &mut &expected_result[..])?;
    let mut response = client.execute("RETRIEVE blob")?;
    let mut actual_result: Vec<u8> = vec![];
    response.read_to_end(&mut actual_result)?;
    let (_, info) = response.close()?;

    println!("{:?}", actual_result);
    println!("{}", info);

    assert_eq!(expected_result.to_vec(), actual_result);
    assert!(info.starts_with("Query executed in"));

    Ok(())
}
