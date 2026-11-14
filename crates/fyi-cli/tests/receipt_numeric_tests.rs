use std::path::Path;
use std::process::Command;

#[test]
fn cli_verifies_python_exponent_receipt_without_changing_evidence() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/acquisition/receipt-v1-exponent-python.json");
    let original = std::fs::read(&fixture).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_fyi-cli"))
        .arg("verify-acquisition-receipt")
        .arg(&fixture)
        .output()
        .expect("run offline receipt verifier");
    assert!(output.status.success(), "{:?}", output);
    assert!(String::from_utf8_lossy(&output.stdout).contains("Acquisition receipt verified"));
    assert_eq!(std::fs::read(&fixture).unwrap(), original);
}
