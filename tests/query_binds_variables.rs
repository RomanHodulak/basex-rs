use test_case::test_case;
use std::net::IpAddr;
use std::io::Read;
use basex;
use basex::{Client, ClientError, ToQueryArgument};

#[test_case(IpAddr::V4("125.0.0.1".parse().unwrap()), "125.0.0.1", "xs:string")]
#[test_case("test", "test", "xs:string")]
#[test_case("test".to_owned(), "test", "xs:string")]
#[test_case(5u8, "5", "xs:unsignedByte")]
#[test_case(5u16, "5", "xs:unsignedShort")]
#[test_case(5u32, "5", "xs:unsignedInt")]
#[test_case(5u64, "5", "xs:unsignedLong")]
#[test_case(5i8, "5", "xs:byte")]
#[test_case(5i16, "5", "xs:short")]
#[test_case(5i32, "5", "xs:int")]
#[test_case(5i64, "5", "xs:long")]
#[test_case(true, "true", "xs:boolean")]
#[test_case(5.5f32, "5.5", "xs:float")]
#[test_case(5.5f64, "5.5", "xs:double")]
#[test_case(&5.2f64, "5.2", "xs:double")]
#[test_case(Some(true), "true", "xs:boolean")]
fn test_query_binds_variables<'a, T: 'a + ToQueryArgument<'a>>(
    value: T,
    expected_result: &str,
    expected_type: &str
) -> Result<(), ClientError> {
    let mut client = Client::connect("localhost", 1984, "admin", "admin")?;

    let database_name = "525fd16";
    let info = client.create(database_name)?.without_input()?;

    assert!(info.starts_with(&format!("Database '{}' created", database_name)));

    let mut response = {
        let mut query = client.query(
            &mut format!("declare variable $prdel as {} external; $prdel", expected_type).as_bytes()
        )?.without_info()?;
        query.bind("prdel")?.with_value(value)?;
        query.execute()?
    };
    let mut actual_result = String::new();
    response.read_to_string(&mut actual_result)?;
    response.close()?.close()?;

    assert_eq!(expected_result, actual_result);
    Ok(())
}
