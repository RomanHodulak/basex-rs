use basex::{Client, ClientError};
use std::fs::File;

macro_rules! path {
    ($path:expr) => {
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/home_depot/files/", $path))
    };
}

fn main() -> Result<(), ClientError> {
    let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
    let mut catalog = File::open(path!("catalog.xml"))?;
    let mut warehouse = File::open(path!("warehouse.xml"))?;
    let mut xquery = File::open(path!("hornbach.xq"))?;

    let info = client.create("hornbach")?.without_input()?;
    assert!(info.starts_with("Database 'hornbach' created"));

    let info = client.add("catalog", &mut catalog)?;
    assert!(info.starts_with("Resource(s) added"));

    let info = client.add("warehouse", &mut warehouse)?;
    assert!(info.starts_with("Resource(s) added"));

    let mut query = client.query(&mut xquery)?;
    let result = query.execute()?;
    let _ = query.close()?;

    println!("{}", result);

    Ok(())
}
