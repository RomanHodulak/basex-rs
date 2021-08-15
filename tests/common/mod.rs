use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "tests/files/"]
#[prefix = ""]
pub struct Asset;
