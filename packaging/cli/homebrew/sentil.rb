class Sentil < Formula
  desc "Runtime verification for Signal Temporal Logic and PrSTL"
  homepage "https://github.com/sedislab/SENTIL"
  version "0.3.0"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
      url "https://github.com/sedislab/SENTIL/releases/download/v0.3.0/sentil-0.3.0-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_AARCH64_DARWIN_SHA256"
    end
    on_intel do
      url "https://github.com/sedislab/SENTIL/releases/download/v0.3.0/sentil-0.3.0-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_X86_64_DARWIN_SHA256"
    end
  end

  on_linux do
    url "https://github.com/sedislab/SENTIL/releases/download/v0.3.0/sentil-0.3.0-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "REPLACE_WITH_X86_64_LINUX_SHA256"
  end

  def install
    prefix.install Dir["*"]
    bin.install_symlink prefix/"bin/sentil"
    man1.install prefix/"man/sentil.1" if (prefix/"man/sentil.1").exist?
    bash_completion.install prefix/"completions/sentil.bash" => "sentil" if (prefix/"completions/sentil.bash").exist?
    zsh_completion.install prefix/"completions/_sentil" if (prefix/"completions/_sentil").exist?
    fish_completion.install prefix/"completions/sentil.fish" if (prefix/"completions/sentil.fish").exist?
  end

  test do
    assert_match "sentil", shell_output("#{bin}/sentil --version")
  end
end