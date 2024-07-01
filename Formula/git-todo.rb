# Documentation: https://docs.brew.sh/Formula-Cookbook
#                https://rubydoc.brew.sh/Formula
# PLEASE REMOVE ALL GENERATED COMMENTS BEFORE SUBMITTING YOUR PULL REQUEST!
class GitTodo < Formula
  desc "A git todo extension"
  homepage "https://github.com/dspo/homebrew-git-todo"
  url "https://github.com/dspo/homebrew-git-todo/archive/refs/tags/v0.0.1.tar.gz"
  sha256 "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  version "0.0.1"
  depends_on "rust" => :build

  def install
    system "cargo", "build", "--release", "--bin", "git-todo"
    bin.install "target/release/git-todo"
  end
end
