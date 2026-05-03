{pkgs, ...}: {
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
    cargo build || exit 1

    echo "Use Ctrl-A X to kill"
    qemu-system-riscv64 \
      -machine virt \
      -cpu rv64 \
      -nographic \
      -bios default \
      -serial mon:stdio \
      --no-reboot \
      -kernel target/riscv64gc-unknown-none-elf/debug/nios-kernel
  '';

  scripts.check.exec = ''
    cargo build --tests || exit 1

    TEST_BIN=$(cargo build --tests --message-format=json | jq -r 'select(.executable != null) | .executable')
    qemu-system-riscv64 \
      -machine virt \
      -cpu rv64 \
      -nographic \
      -bios default \
      -serial mon:stdio \
      --no-reboot \
      -kernel $TEST_BIN
  '';
}
