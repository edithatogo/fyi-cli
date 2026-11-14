# Review packet for a maintainer tap, not Homebrew core.
# Historical binding: GitHub v1.2.0 contains Rust component 0.1.2.
# Publication is blocked by packaging/release-evidence.json.
class FyiCli < Formula
  desc "Privacy-focused multi-jurisdiction FOI/OIA CLI for Alaveteli instances"
  homepage "https://github.com/edithatogo/fyi-cli"
  version "0.1.2"
  license "MIT"

  on_macos do
    on_intel do
      url "https://github.com/edithatogo/fyi-cli/releases/download/v1.2.0/fyi-cli-macos-amd64.tar.gz"
      sha256 "55502bd634e285345c94abac34a049885a38671f3c496c2b4a063a5fd13493c1"
    end
    on_arm do
      url "https://github.com/edithatogo/fyi-cli/releases/download/v1.2.0/fyi-cli-macos-arm64.tar.gz"
      sha256 "e254515e22dfdf8f2726a452516e8241d956f54e1401bf66a396e6b3e5eb7e9f"
    end
  end

  on_linux do
    depends_on arch: :x86_64
    url "https://github.com/edithatogo/fyi-cli/releases/download/v1.2.0/fyi-cli-linux-amd64.tar.gz"
    sha256 "af89253ae418c95e62c13c64c41f1a1f2b02a780e47fe016776211f2d7e0520a"
  end

  def install
    bin.install "fyi-cli"
    bin.install_symlink "fyi-cli" => "fyi"
  end

  test do
    system "#{bin}/fyi-cli", "--help"
  end
end
