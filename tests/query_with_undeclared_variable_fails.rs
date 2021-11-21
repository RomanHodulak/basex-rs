mod common;

use basex;
use basex::{Client, ClientError};

#[test]
fn test_query_with_undeclared_variable_fails() -> Result<(), ClientError> {
    let client = Client::connect("localhost", 1984, "admin", "admin")?;

    let query = client.query("$x")?;
    let actual_error = query.execute()?.close().err().unwrap();
    assert!(matches!(actual_error, ClientError::QueryFailed(_)));

    if let ClientError::QueryFailed(q) = actual_error {
        assert_eq!("Stopped at ., 1/1:\n[XPST0008] Undeclared variable: $x.", q.raw());
        assert_eq!("Undeclared variable: $x.", q.message());
        assert_eq!(1, q.line());
        assert_eq!(1, q.position());
        assert_eq!(".", q.file());
        assert_eq!("XPST0008", q.code());
    }
    Ok(())
}
