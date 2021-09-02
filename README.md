# BaseX

[![Build Status](https://github.com/RomanHodulak/basex-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/RomanHodulak/basex-rs/actions)
[![Code Coverage](https://codecov.io/gh/RomanHodulak/basex-rs/branch/master/graph/badge.svg?token=GDG9C63SNE)](https://codecov.io/gh/RomanHodulak/basex-rs)
[![Current Crates.io Version](https://img.shields.io/crates/v/basex.svg)](https://crates.io/crates/basex)

A client library for BaseX databases. Compatible with 8.x, 9.x+.

## Usage
```rust
use basex::Client;

fn main() {
    let mut client = Client::connect("localhost", 1984, "admin", "admin").unwrap();

    let info = client.create("lambada", Some("<None><Text></Text><Lala></Lala><Papa></Papa></None>")).unwrap();
    assert!(info.starts_with("Database 'lambada' created"));

    let mut query = client.query("count(/None/*)").unwrap();
    let result = query.execute().unwrap();
    assert_eq!(result, "3");

    let _ = query.close().unwrap();
}
```
