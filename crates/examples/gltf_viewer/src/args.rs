use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Path of the glTF file
    #[clap(short, long, value_parser, default_value = "")]
    pub file: String,
}
