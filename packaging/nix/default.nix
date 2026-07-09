# nixpkgs-style expression for fyi-cli / fyi-mcp.
# Draft: place under pkgs/by-name/fy/fyi-cli/package.nix (or similar) when submitting
# upstream. Prefer cargoHash update after first successful nix-build.
#
# Usage (local flake / nix-build -E):
#   nix-build -E 'with import <nixpkgs> {}; callPackage ./default.nix {}'
#
# Homepage: https://github.com/edithatogo/fyi-cli
# License: MIT
# Version: 0.1.2

{
  lib,
  rustPlatform,
  fetchFromGitHub,
  pkg-config,
  openssl,
  stdenv,
}:

rustPlatform.buildRustPackage rec {
  pname = "fyi-cli";
  version = "0.1.2";

  src = fetchFromGitHub {
    owner = "edithatogo";
    repo = "fyi-cli";
    rev = "v${version}";
    # Update with: nix-prefetch-github edithatogo fyi-cli --rev v0.1.2
    hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
  };

  # Set after first failed build prints the expected hash, e.g.:
  #   nix-build -E 'with import <nixpkgs> {}; callPackage ./default.nix {}'
  cargoHash = lib.fakeHash;

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [
    openssl
  ] ++ lib.optionals stdenv.hostPlatform.isDarwin [
    # darwin.apple_sdk.frameworks.Security
  ];

  # Build CLI and MCP server binaries from the workspace.
  buildAndTestSubdir = null;
  cargoBuildFlags = [
    "--package"
    "fyi-cli"
    "--package"
    "fyi-mcp"
  ];

  # Network-heavy integration tests are skipped in sandbox builds.
  doCheck = false;

  meta = with lib; {
    description = "Privacy-focused multi-jurisdiction FOI/OIA CLI and MCP server for Alaveteli";
    longDescription = ''
      fyi-cli tracks and manages Freedom of Information / Official Information
      requests against Alaveteli-based platforms (FYI.org.nz and other instances),
      with a Rust CLI, MCP server, optional Tor support, and local SQLite storage.
    '';
    homepage = "https://github.com/edithatogo/fyi-cli";
    changelog = "https://github.com/edithatogo/fyi-cli/blob/v${version}/CHANGELOG.md";
    license = licenses.mit;
    maintainers = with maintainers; [
      # add nixpkgs maintainer handle when submitting
    ];
    mainProgram = "fyi-cli";
    platforms = platforms.unix ++ platforms.windows;
  };
}
