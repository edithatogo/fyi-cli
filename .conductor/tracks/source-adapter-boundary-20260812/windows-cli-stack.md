# Windows CLI startup stack — #310 portability finding

## Reproduced failure

PR #430 head `8fc2fae` fixes fixture byte conversion. Its Windows job
`132295643211` in portability run `43545466384` passed all core receipt suites,
including numeric/midpoint, strict JSON, symlink and independent-process tests.
The real CLI test then failed immediately with Windows status `3221225725`
(`0xC00000FD`) and `thread 'main' has overflowed its stack`, before producing a
verification verdict. The same CLI path passed on Linux and macOS. This is a
separate executable-startup failure, not a reason to skip Windows verification.

## Bounded correction and acceptance

Add `crates/fyi-cli/build.rs` to reserve **8 MiB** of main-thread stack for the
`fyi-cli` Windows MSVC executable. Leave the initial commit amount at its default;
this reserves address space, not an eager 8 MiB physical allocation. Cargo's
binary-scoped link argument affects the shipped executable in every profile,
not dependency libraries or a test-only thread. Target configuration, rather
than the build host, selects the flag. Other targets are unchanged.

The monolithic unoptimised CLI dispatch is the suspected source of stack demand;
its exact frame contribution is not yet measured. Splitting large dispatch/future
frames remains a useful refactor, not a claim made by this fix. A bounded executable
reservation is preferable here to a CI-only environment override, test omission,
or changing the JSON recursion policy. The original failing receipt CLI test is
retained unchanged.

- [x] Preserve the Windows red execution above and original receipt assertions.
- [~] Add startup controls for `--help` and `--version` with `RUST_MIN_STACK` removed.
- [~] Inspect the actual Windows executable's PE header in a Rust integration test:
  verify exactly 8 MiB reserve and a smaller positive initial commit amount.
- [ ] Rerun all three platforms and the complete required final-head CI before merge.

This does not demonstrate Windows GNU/ARM runtime coverage, arbitrary command
stack bounds or power-loss durability. No live data or external service is used
by the startup tests. The wider acceptance requirements of issue #310 remain active.

## Primary references

- Microsoft `/STACK` linker option: https://learn.microsoft.com/en-us/cpp/build/reference/stack-stack-allocations
- Microsoft PE format, SizeOfStackReserve/Commit: https://learn.microsoft.com/en-us/windows/win32/debug/pe-format
- Cargo binary-scoped linker arguments: https://doc.rust-lang.org/cargo/reference/build-scripts.html#rustc-link-arg-bin
- Rust thread documentation distinguishes the process main stack from `RUST_MIN_STACK`: https://doc.rust-lang.org/std/thread/index.html#stack-size
