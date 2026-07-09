class FyiCli < Formula
  desc "Privacy-focused multi-jurisdiction FOI/OIA CLI for Alaveteli instances"
  homepage "https://github.com/edithatogo/fyi-cli"
  version "0.1.2"
  license "MIT"

  # Update url/sha256 from the matching GitHub Release (cargo-dist or manual).
  if OS.mac?
    if Hardware::CPU.intel?
      url "https://github.com/edithatogo/fyi-cli/releases/download/v#{version}/fyi-cli-macos-amd64.tar.gz"
      sha256 "PLACEHOLDER_MAC_AMD64_SHA256"
    elsif Hardware::CPU.arm?
      url "https://github.com/edithatogo/fyi-cli/releases/download/v#{version}/fyi-cli-macos-arm64.tar.gz"
      sha256 "PLACEHOLDER_MAC_ARM64_SHA256"
    end
  elsif OS.linux?
    url "https://github.com/edithatogo/fyi-cli/releases/download/v#{version}/fyi-cli-linux-amd64.tar.gz"
    sha256 "PLACEHOLDER_LINUX_AMD64_SHA256"
  end

  def install
    bin.install "fyi-cli" => "fyi"
  end

  test do
    system "#{bin}/fyi", "--help"
  end
end
