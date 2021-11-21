<div style="text-align: center">

# ![BaseX Logo](https://basex.org/images/basex.svg "BaseX")

[![Build Status](https://github.com/RomanHodulak/basex-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/RomanHodulak/basex-rs/actions)
[![Code Coverage](https://codecov.io/gh/RomanHodulak/basex-rs/branch/master/graph/badge.svg?token=GDG9C63SNE)](https://codecov.io/gh/RomanHodulak/basex-rs)
[![Current Crates.io Version](https://img.shields.io/crates/v/basex.svg)](https://crates.io/crates/basex)
[![Documentation](https://docs.rs/basex/badge.svg)](https://docs.rs/basex)
[![Current Crates.io Downloads](https://img.shields.io/crates/d/basex.svg)](https://crates.io/crates/basex)

</div>

This library is a client implementation of the open-source XML database server and XQuery processor [BaseX](http://basex.org).

Compatible with versions 8.x and 9.x.

## Installation
Add the library to the list of dependencies in your `Cargo.toml` like so:

```toml
[dependencies]
basex = "0.5.0"
```

## Usage

### 1. Set up a database server
First, you need to have BaseX server up and running. If you want to try it out, you can do it right away using docker.

```shell
docker run -p 1984:1984 basex/basexhttp:9.5.2
```

Every example can be run with this server configuration.

### 2. Connect to the server
Before you can do anything with the database server, you need to establish connection and authorize. Typically, you do this by calling `Client::connect`. If you get Ok result, you get the instance of the `Client`. Having its instance guarantees to have an open session with the server.

```rust
let client = Client::connect("localhost", 1984, "admin", "admin")?;
```

You can now send commands.

### 3. Open database
To run a query, you need to open a database.

#### 3.1. Create a new database
Creating a database also opens it. Follow the create call with either `without_input` or `with_input` to optionally specify initial XML resource.

```rust
let info = client.create("coolbase")?.with_input(&mut xml)?;
```

#### 3.2. Open an existing database
Use `Client::execute` with command [`OPEN [name]`](https://docs.basex.org/wiki/Commands#OPEN).

```rust
let (client, info) = client.execute("OPEN coolbase")?.close()?;
```

### 4. Run queries
Aside from running commands, you can run queries using XQuery syntax which is the most important use-case.

1. Create a new query using `Client::query`. This puts the session into query mode. 
2. Optionally, bind arguments using `Query::bind`. 
3. Execute it using `Query::execute`.
4. Close the query using `Query::close`.

## Example
The following example creates database "lambada" with initial XML resource and counts all first-level child nodes of the `Root` node.

```rust
use basex::{Client, ClientError};
use std::io::Read;

fn main() -> Result<(), ClientError> {
    let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
    let info = client.create("lambada")?
        .with_input("<Root><Text/><Lala/><Papa/></Root>")?;
    assert!(info.starts_with("Database 'lambada' created"));

    let query = client.query("count(/Root/*)")?;

    let mut result = String::new();
    let mut response = query.execute()?;
    response.read_to_string(&mut result)?;
    assert_eq!(result, "3");

    let mut query = response.close()?;
    query.close()?;
    Ok(())
}
```

## License
The library is licensed under [ISC](LICENSE).
