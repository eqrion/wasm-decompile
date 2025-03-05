use std::ffi::OsStr;

#[test]
fn test_snapshot() {
    let update_snapshots = std::env::var("UPDATE_SNAPSHOTS").is_ok();

    let test_files = std::fs::read_dir("tests/snapshots").unwrap();
    for file in test_files {
        let file = file.unwrap();

        let test_path = file.path();
        if test_path.extension() != Some(OsStr::new("wat")) {
            continue;
        }

        let input = std::fs::read(&test_path).unwrap();
        let input_binary = wat::parse_bytes(&input).unwrap();
        let module = wasm_decompile::Module::from_buffer(&input_binary).unwrap();
        let mut output = Vec::new();
        module.write(&mut output).unwrap();
        let output_string = String::from_utf8(output).unwrap();

        let expected_path = test_path.with_extension("snapshot");
        if update_snapshots {
            std::fs::write(expected_path, output_string).unwrap();
        } else {
            let expected = std::fs::read(&expected_path).unwrap();
            assert_eq!(output_string, String::from_utf8(expected).unwrap());
        }
    }
}
