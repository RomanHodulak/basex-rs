mod common;

use basex;
use basex::{Client, ClientError};

#[test]
fn test_query_recognizes_updating_statements() -> Result<(), ClientError> {
    let client = Client::connect("localhost", 1984, "admin", "admin")?;

    let mut non_updating_query = client.query("count(/None/*)")?;
    assert!(!non_updating_query.updating()?);

    let client = non_updating_query.close()?;

    let mut updating_query = client.query("replace value of node /None with 1")?;
    assert!(updating_query.updating()?);

    updating_query.close()?;
    Ok(())
}
