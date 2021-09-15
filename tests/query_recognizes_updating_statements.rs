mod common;

use basex;
use basex::{Client, ClientError};

#[test]
fn test_query_recognizes_updating_statements() -> Result<(), ClientError> {
    let mut client = Client::connect("localhost", 1984, "admin", "admin")?;

    let mut non_updating_query = client.query(&mut "count(/None/*)".as_bytes())?;
    assert!(!non_updating_query.updating()?);

    let mut updating_query = client.query(&mut "replace value of node /None with 1".as_bytes())?;
    assert!(updating_query.updating()?);

    non_updating_query.close()?;
    updating_query.close()?;
    Ok(())
}