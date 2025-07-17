#![no_main]

use libfuzzer_sys::fuzz_target;

use arbitrary::Unstructured;
use wasm_smith::Module as SmithModule;
use wasm_decompile::Module as DecompileModule;

fuzz_target!(|bytes: Vec<u8>| {
    let mut u = Unstructured::new(&bytes);
    let config = wasm_smith::Config {
        reference_types_enabled: false,
        gc_enabled: false,
        exceptions_enabled: false,

        multi_value_enabled: false,
        bulk_memory_enabled: false,
        memory64_enabled: false,
        relaxed_simd_enabled: false,
        simd_enabled: false,
        tail_call_enabled: false,
        threads_enabled: false,
        wide_arithmetic_enabled: false,
        custom_page_sizes_enabled: false,

        saturating_float_to_int_enabled: true,
        sign_extension_ops_enabled: true,
        min_funcs: 1,
        max_imports: 0,
        ..wasm_smith::Config::default()
    };
    let module = SmithModule::new(config, &mut u).unwrap();
    let wasm_bytes = module.to_bytes();
    // println!("{}", wasmprinter::print_bytes(&wasm_bytes).unwrap());
    let module = DecompileModule::from_buffer(&wasm_bytes).unwrap();
    let mut output = Vec::new();
    module.write(&mut output).unwrap();
});
