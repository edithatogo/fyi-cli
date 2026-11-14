use std::path::Path;
use std::process::Command;

#[test]
fn cli_verifies_python_numeric_receipts_without_changing_evidence() {
    for name in [
        "receipt-v1-exponent-python.json",
        "receipt-v1-midpoints-python.json",
    ] {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../conformance/acquisition")
            .join(name);
        let original = std::fs::read(&fixture).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_fyi-cli"))
            .arg("verify-acquisition-receipt")
            .arg(&fixture)
            .output()
            .expect("run offline receipt verifier");
        assert!(output.status.success(), "{name}: {output:?}");
        assert!(String::from_utf8_lossy(&output.stdout).contains("Acquisition receipt verified"));
        assert_eq!(std::fs::read(&fixture).unwrap(), original);
    }
}
