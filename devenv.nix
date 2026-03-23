{
  pkgs,
  lib,
  config,
  inputs,
  ...
}: {
  # https://devenv.sh/packages/
  packages = [pkgs.qemu];

  # https://devenv.sh/languages/
  languages.rust = {
    enable = true;
    channel = "nightly";
    targets = ["riscv64gc-unknown-none-elf"];
    components = ["rustc" "cargo" "clippy" "rustfmt" "rust-src"];
  };

  scripts.run.exec = ''
    cargo build &&
    echo "Use Ctrl-A X to kill" &&
    qemu-system-riscv64 \
      -machine virt \
      -cpu rv64 \
      -nographic \
      -bios none \
      -kernel target/riscv64gc-unknown-none-elf/debug/nios \
      -serial mon:stdio
  '';
}
