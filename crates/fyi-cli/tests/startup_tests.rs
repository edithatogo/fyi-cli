//! Exercise the real CLI entry point without a test-only stack override.
use std::process::Command;

#[test]
fn help_and_version_start_without_thread_stack_environment() {
    for argument in ["--help", "--version"] {
        let output = Command::new(env!("CARGO_BIN_EXE_fyi-cli"))
            .arg(argument)
            .env_remove("RUST_MIN_STACK")
            .output()
            .expect("start the actual CLI executable");
        assert!(output.status.success(), "{argument}: {output:?}");
        assert!(String::from_utf8_lossy(&output.stdout).contains("fyi-cli"));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("overflowed"));
    }
}

#[cfg(all(target_os = "windows", target_env = "msvc"))]
#[test]
fn windows_executable_has_bounded_stack_reserve_without_eager_commit() {
    // PE optional-header offsets are specified by Microsoft's PE/COFF format.
    // Inspect the built application, not this test harness or a source string.
    let image = std::fs::read(env!("CARGO_BIN_EXE_fyi-cli")).unwrap();
    assert_eq!(&image[..2], b"MZ");
    let pe = u32::from_le_bytes(image[0x3c..0x40].try_into().unwrap()) as usize;
    assert_eq!(&image[pe..pe + 4], b"PE\0\0");
    let optional = pe + 24;
    let magic = u16::from_le_bytes(image[optional..optional + 2].try_into().unwrap());
    let (reserve, commit) = match magic {
        0x20b => (
            u64::from_le_bytes(image[optional + 72..optional + 80].try_into().unwrap()),
            u64::from_le_bytes(image[optional + 80..optional + 88].try_into().unwrap()),
        ),
        0x10b => (
            u64::from(u32::from_le_bytes(
                image[optional + 72..optional + 76].try_into().unwrap(),
            )),
            u64::from(u32::from_le_bytes(
                image[optional + 76..optional + 80].try_into().unwrap(),
            )),
        ),
        _ => panic!("unexpected PE optional-header magic: {magic:#x}"),
    };
    assert_eq!(reserve, 8 * 1024 * 1024);
    assert!(
        commit > 0 && commit < reserve,
        "unexpected initial stack allocation: commit={commit}, reserve={reserve}"
    );
}
