# BLOCKED TEMPLATE: fill both independently computed Nix hashes before building.
# Historical source v1.2.0 declares Rust version 0.1.2; reconcile this mismatch
# before submission. Do not confuse this source with unreleased master.
# Pin the nixpkgs revision/toolchain in the eventual build receipt; then use
# nix-prefetch-github for src.hash and the fixed-output vendor derivation for
# cargoHash. Native build, dependency licences and maintainer review remain open.

{ lib, rustPlatform, fetchFromGitHub, pkg-config, openssl }:

rustPlatform.buildRustPackage rec {
  pname = "fyi-cli";
  version = "0.1.2";

  src = fetchFromGitHub {
    owner = "edithatogo";
    repo = "fyi-cli";
    rev = "86fcbd8914037735b6f27e5a329d5f3013033efd";
    hash = "@SOURCE_NAR_HASH@";
  };

  cargoHash = "@CARGO_VENDOR_HASH@";
  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ openssl ];
  cargoBuildFlags = [ "--package" "fyi-cli" "--package" "fyi-mcp" ];
  cargoTestFlags = [ "--package" "fyi-cli" "--package" "fyi-mcp" ];
  doCheck = true;

  meta = {
    description = "Privacy-focused multi-jurisdiction FOI/OIA CLI and MCP server for Alaveteli";
    homepage = "https://github.com/edithatogo/fyi-cli";
    changelog = "https://github.com/edithatogo/fyi-cli/blob/86fcbd8914037735b6f27e5a329d5f3013033efd/CHANGELOG.md";
    license = lib.licenses.mit;
    # Add the verified nixpkgs maintainer handle before upstream submission.
    maintainers = [ ];
    mainProgram = "fyi-cli";
    platforms = [ "x86_64-linux" "aarch64-linux" ];
  };
}
