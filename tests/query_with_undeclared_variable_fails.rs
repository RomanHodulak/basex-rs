mod common;

use basex;
use basex::{Client, ClientError};

#[test]
fn test_query_with_undeclared_variable_fails() -> Result<(), ClientError> {
    let client = Client::connect("localhost", 1984, "admin", "admin")?;

    let query = client.query(&mut "$x".as_bytes())?;
    let actual_error = query.execute()?.close().err().unwrap();
    match &actual_error {
        ClientError::QueryFailed(q) => {
            assert_eq!("Stopped at ., 1/1:\n[XPST0008] Undeclared variable: $x.", q.raw());
            assert_eq!("Undeclared variable: $x.", q.message());
            assert_eq!(1, q.line());
            assert_eq!(1, q.position());
            assert_eq!(".", q.file());
            assert_eq!("XPST0008", q.code());
        },
        _ => assert!(false),
    };
    Ok(())
}
