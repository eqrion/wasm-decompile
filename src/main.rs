use anyhow::bail;
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
    #[clap(short = 'g')]
    graphviz: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let input = std::fs::read(&cli.input)?;
    let input_binary = wat::parse_bytes(&input)?;
    let module = Module::from_buffer(&input_binary)?;

    let output: Box<dyn std::io::Write> = if let Some(output_path) = cli.output {
        Box::new(std::fs::File::create(&output_path)?)
    } else {
        Box::new(std::io::stdout())
    };

    if let Some(func_index) = cli.func_index {
        if cli.graphviz {
            module.write_func_graphviz(func_index, output)?;
        } else {
            module.write_func(func_index, output)?;
        }
    } else {
        if cli.graphviz {
            bail!("cannot use graphviz on a whole module");
        }
        module.write(output)?;
    }

    Ok(())
}
