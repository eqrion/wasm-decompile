use clap::Parser;
use std::path::PathBuf;

mod ir;
pub use ir::*;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    input: PathBuf,
    output: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let input = std::fs::read(&cli.input)?;
    let input_binary = wat::parse_bytes(&input)?;
    let module = Module::from_buffer(&input_binary)?;

    if let Some(output) = cli.output {
        let output = std::fs::File::create(&output)?;
        module.write(output)?;
    } else {
        module.write(std::io::stdout())?;
    }
    Ok(())
}
