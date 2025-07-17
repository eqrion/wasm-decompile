use clap::Parser;
use std::path::PathBuf;

mod ir;
pub use ir::*;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    input: PathBuf,
    output: Option<PathBuf>,
    #[clap(short = 'f')]
    func_index: Option<u32>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let input = std::fs::read(&cli.input)?;
    let input_binary = wat::parse_bytes(&input)?;
    let module = Module::from_buffer(&input_binary)?;

    if let Some(output) = cli.output {
        let output = std::fs::File::create(&output)?;
        if let Some(func_index) = cli.func_index {
            module.write_func(func_index, output)?;
        } else {
            module.write(output)?;
        }
    } else {
        let output = std::io::stdout();
        if let Some(func_index) = cli.func_index {
            module.write_func(func_index, output)?;
        } else {
            module.write(output)?;
        }
    }
    Ok(())
}
