# nios

A simple operating system written from scratch for RISC-V.

### Roadmap

This is a rough roadmap for the project, it can and will change regularly.
Goal is to build features incrementally, making sure it just works, improving later.

- [x] Kernel init
- [x] Simple trap handling
- [x] Physical Memory Manager
- [x] Kernel Heap (kmalloc)
- [x] UART input (poll LSR, read bytes)
- [x] Shell
  - [x] Line reader (buffered input, backspace handling)
  - [x] Command parser (tokenize input)
  - [x] Built-in commands
- [ ] Virtual Memory and Paging (optional)
- [ ] Multitasking and Context Switching (optional)

### Running

1. With nix

```sh
direnv allow
run
```

This creates the enviroment with devenv and runs the project in qemu virtual machine.

2. Without nix

```sh
cargo build
qemu-system-riscv64 \
  -machine virt \
  -cpu rv64 \
  -nographic \
  -bios none \
  -kernel target/riscv64gc-unknown-none-elf/debug/nios \
  -serial mon:stdio
```

#### Running tests

1. With nix

```sh
direnv allow
check
```

This creates the enviroment with devenv and runs the tests in qemu virtual machine.

2. Without nix

```sh
TEST_BIN=$(cargo build --tests --message-format=json | jq -r 'select(.executable != null) | .executable')
qemu-system-riscv64 \
  -machine virt \
  -cpu rv64 \
  -nographic \
  -bios none \
  -kernel $TEST_BIN \
  -serial mon:stdio
```

Those tests need to run on RISC-V machine, therefore vm is required for testing.
The `--message-format` flag is needed to get the name of test executable that `cargo build` produces, as it needs to be loaded as a kernel into qemu vm.
