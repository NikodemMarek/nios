# nios

A simple operating system written from scratch for RISC-V.

### Roadmap

This is a rough roadmap for the project, it can and will change regularly.
Goal is to build features incrementally, making sure it just works, improving later.

- [x] Kernel init
- [x] Simple trap handling
- [x] Physical Memory Manager
- [x] Kernel Heap (kmalloc)
- [ ] UART input (poll LSR, read bytes)
- [ ] Shell
  - [ ] Line reader (buffered input, backspace handling)
  - [ ] Command parser (tokenize input)
  - [ ] Built-in commands
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
