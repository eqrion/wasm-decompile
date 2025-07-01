#[allow(warnings)]
mod bindings;

use bindings::{Guest, PrintPart};
use wasm_decompile::*;
use wat;

struct Component;

struct StringWriter(pub Vec<PrintPart>);

impl wasmprinter::Print for StringWriter {
    fn write_str(&mut self, s: &str) -> std::io::Result<()> {
        self.0.push(PrintPart::Str(s.to_string()));
        Ok(())
    }

    fn start_name(&mut self) -> std::io::Result<()> {
        self.0.push(PrintPart::Name);
        Ok(())
    }

    fn start_literal(&mut self) -> std::io::Result<()> {
        self.0.push(PrintPart::Literal);
        Ok(())
    }

    fn start_keyword(&mut self) -> std::io::Result<()> {
        self.0.push(PrintPart::Keyword);
        Ok(())
    }

    fn start_type(&mut self) -> std::io::Result<()> {
        self.0.push(PrintPart::Type);
        Ok(())
    }

    fn start_comment(&mut self) -> std::io::Result<()> {
        self.0.push(PrintPart::Comment);
        Ok(())
    }

    fn reset_color(&mut self) -> std::io::Result<()> {
        self.0.push(PrintPart::Reset);
        Ok(())
    }
}

impl Guest for Component {
    fn parse(contents: String) -> Result<Vec<u8>, String> {
        wat::parse_str(contents).map_err(|e| e.to_string())
    }

    fn print(contents: Vec<u8>, skeleton: bool) -> Result<Vec<PrintPart>, String> {
        let mut config = wasmprinter::Config::new();
        config.print_skeleton(skeleton);

        let mut writer = StringWriter(Vec::new());
        let result = config.print(&contents, &mut writer);

        result.map(|_| writer.0).map_err(|e| e.to_string())
    }

    fn print_decompiled(contents: Vec<u8>) -> Result<String, String> {
        let input_binary = wat::parse_bytes(&contents).map_err(|e| e.to_string())?;
        let module = Module::from_buffer(&input_binary).map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        module.write(&mut out).map_err(|x| x.to_string())?;
        String::from_utf8(out).map_err(|x| x.to_string())
    }
}

bindings::export!(Component with_types_in bindings);
