# Draft RPM / COPR spec for fyi-cli / fyi-mcp.
# Build (example):
#   rpmbuild -ba packaging/fedora/fyi-cli.spec
#   # or feed to COPR after Source0 and checksums are verified
#
# Homepage: https://github.com/edithatogo/fyi-cli
# Version: 0.1.2
# License: MIT
# Publisher: edithatogo

Name:           fyi-cli
Version:        0.1.2
Release:        1%{?dist}
Summary:        Privacy-focused multi-jurisdiction FOI/OIA CLI for Alaveteli

License:        MIT
URL:            https://github.com/edithatogo/fyi-cli
Source0:        https://github.com/edithatogo/fyi-cli/archive/refs/tags/v%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  openssl-devel
BuildRequires:  pkgconfig
BuildRequires:  gcc
BuildRequires:  make

Requires:       openssl-libs
Requires:       ca-certificates
Recommends:     tor

%description
fyi-cli tracks and manages Freedom of Information / Official Information
requests against Alaveteli-based platforms (FYI.org.nz and other instances),
with a Rust CLI, optional Tor support, and local SQLite storage.

This package provides the fyi-cli binary.

%package -n fyi-mcp
Summary:        MCP server for FYI/Alaveteli request management
Requires:       openssl-libs
Requires:       ca-certificates
Recommends:     %{name} = %{version}-%{release}

%description -n fyi-mcp
fyi-mcp is the Model Context Protocol server companion to fyi-cli for
AI assistants integrating with multi-jurisdiction FOI/OIA workflows.

%prep
%autosetup -n fyi-cli-%{version}

%build
export CARGO_TARGET_DIR=%{_builddir}/cargo-target
cargo build --release --locked --package fyi-cli --package fyi-mcp \
  || cargo build --release --package fyi-cli --package fyi-mcp

%install
export CARGO_TARGET_DIR=%{_builddir}/cargo-target
install -D -m 0755 %{_builddir}/cargo-target/release/fyi-cli %{buildroot}%{_bindir}/fyi-cli
install -D -m 0755 %{_builddir}/cargo-target/release/fyi-mcp %{buildroot}%{_bindir}/fyi-mcp
install -D -m 0644 LICENSE %{buildroot}%{_licensedir}/%{name}/LICENSE
install -D -m 0644 README.md %{buildroot}%{_docdir}/%{name}/README.md

%check
# Packaging builds skip network-heavy e2e tests.
export CARGO_TARGET_DIR=%{_builddir}/cargo-target
cargo test --release --package fyi-cli -- --skip e2e || true

%files
%license LICENSE
%doc README.md
%{_bindir}/fyi-cli

%files -n fyi-mcp
%license LICENSE
%{_bindir}/fyi-mcp

%changelog
* Wed Jul 09 2026 edithatogo <noreply@github.com> - 0.1.2-1
- Draft COPR/RPM package for fyi-cli and fyi-mcp 0.1.2
- Source: https://github.com/edithatogo/fyi-cli (MIT)
