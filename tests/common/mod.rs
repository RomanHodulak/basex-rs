use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "tests/files/"]
pub struct Asset;
