use std::{fs::read_to_string, io, path::PathBuf};

use clap::Parser;
use persistent_kv::{Config, PersistentKeyValueStore};

type Store = PersistentKeyValueStore<String, String>;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Optional name to operate on
    #[arg(short, long)]
    input: PathBuf,

    /// Sets a custom config file
    #[arg(short, long)]
    out: PathBuf,

    /// Store key
    #[arg(short, long)]
    key: String,
}

fn main() -> Result<(), io::Error> {
    let cli = Cli::parse();

    let input_json = read_to_string(cli.input)?;

    let store = Store::new(cli.out, Config::default()).expect("Could not create store");

    let _ = store.set(cli.key, input_json);

    Ok(())
}
