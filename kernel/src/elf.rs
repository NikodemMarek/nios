use crate::memory_manager::{PageTable, PhysicalAddress, Pmm, VirtualAddress};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Header {
    pub ident: [u8; 16],
    pub r#type: u16,
    pub machine: u16,
    pub version: u32,
    pub entry: u64,
    pub phoff: u64,
    pub shoff: u64,
    pub flags: u32,
    pub ehsize: u16,
    pub phentsize: u16,
    pub phnum: u16,
    pub shentsize: u16,
    pub shnum: u16,
    pub shstrndx: u16,
}
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Phdr {
    pub r#type: u32,
    pub flags: u32,
    pub offset: u64,
    pub vaddr: u64,
    pub paddr: u64,
    pub filesz: u64,
    pub memsz: u64,
    pub align: u64,
}

pub const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];
pub const PT_LOAD: u32 = 1;

pub fn load(pmm: &mut Pmm, pt: &mut PageTable, elf_bytes: &[u8]) -> u64 {
    assert!(elf_bytes.len() >= 64, "ELF header too small");
    assert_eq!(&elf_bytes[0..4], &ELF_MAGIC, "Invalid ELF magic");

    let header = unsafe { core::ptr::read_unaligned(elf_bytes.as_ptr() as *const Elf64Header) };

    for i in 0..header.phnum as usize {
        let offset = header.phoff as usize + i * header.phentsize as usize;
        assert!(
            elf_bytes.len() >= offset + header.phentsize as usize,
            "Program header truncated"
        );

        let phdr = unsafe {
            core::ptr::read_unaligned(elf_bytes.as_ptr().add(offset) as *const Elf64Phdr)
        };

        if PT_LOAD == phdr.r#type {
            let src = &elf_bytes[phdr.offset as usize..phdr.offset as usize + phdr.filesz as usize];
            let src_addr = VirtualAddress(src.as_ptr() as usize);
            let dst_addr = PhysicalAddress(phdr.vaddr as usize);

            let dst_end_addr = PhysicalAddress(dst_addr.0 + phdr.filesz as usize);
            pt.map_range(pmm, dst_addr, dst_end_addr);

            unsafe {
                core::ptr::copy_nonoverlapping(
                    src_addr.0 as *const u8,
                    dst_addr.virt().0 as *mut u8,
                    phdr.filesz as usize,
                );
                if phdr.memsz > phdr.filesz {
                    core::ptr::write_bytes(
                        dst_end_addr.virt().0 as *mut u8,
                        0,
                        phdr.memsz as usize - phdr.filesz as usize,
                    );
                }
            }
        }
    }

    header.entry
}
