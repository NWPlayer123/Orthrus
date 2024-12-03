/// This module provides functionality to load Portable Executable (PE) files, normally denoted with a
/// .exe file extension.
use bitflags::bitflags;
use zerocopy::{
    BigEndian, FromBytes, Immutable, KnownLayout, LittleEndian, TryFromBytes, Unaligned, U16, U32,
};

#[derive(PartialEq, Eq, Clone, Copy, Debug, Default, FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
struct Version {
    major: U16<LittleEndian>,
    minor: U16<LittleEndian>,
}

impl core::fmt::Display for Version {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "v{}.{}", self.major, self.minor)
    }
}

#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
struct MZHeader {
    /// Magic, should be "MZ"/0x4D5A
    e_magic: U16<BigEndian>,
    /// "Count of Bytes on Last Page"
    e_cblp: U16<LittleEndian>,
    /// "Count of Pages", Number of 512-byte sections
    e_cp: U16<LittleEndian>,
    /// "Count of Relocations"
    e_crlc: U16<LittleEndian>,
    /// "Count of Paragraphs in Header"
    e_cparhdr: U16<LittleEndian>,
    /// Minimum extra paragraphs needed
    e_minalloc: U16<LittleEndian>,
    /// Maximum extra paragraphs needed
    e_maxalloc: U16<LittleEndian>,
    /// Initial Stack Segment value
    e_ss: U16<LittleEndian>,
    /// Initial Stack Pointer value
    e_sp: U16<LittleEndian>,
    /// Checksum value
    e_csum: U16<LittleEndian>,
    /// Initial Instruction Pointer value
    e_ip: U16<LittleEndian>,
    /// Initial Code Segment value
    e_cs: U16<LittleEndian>,
    /// "Logical File Address of Relocation Table"
    e_lfarlc: U16<LittleEndian>,
    /// Overlay Number
    e_ovno: U16<LittleEndian>,
    /// Reserved Words 1
    e_res: [U16<LittleEndian>; 4],
    /// OEM identifier
    e_oemid: U16<LittleEndian>,
    /// OEM information (specific to `e_oemid`)
    e_oeminfo: U16<LittleEndian>,
    /// Reserved Words 2
    e_res2: [U16<LittleEndian>; 10],
    /// "Logical File Address of New EXE Header"
    e_lfanew: U32<LittleEndian>,
}

impl MZHeader {
    fn load(input: &[u8], offset: usize) -> Option<&Self> {
        // Check if we have enough bytes to create a header
        if (input.len() - offset) < core::mem::size_of::<Self>() {
            return None;
        }

        // If we do, we can infallibly unwrap since we're Unaligned
        let header = Self::ref_from_bytes(&input[offset..]).unwrap();
        match header.e_magic.get() {
            0x4D5A => Some(header),
            _ => None,
        }
    }
}

#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
struct PEHeader {
    /// Magic, should be "PE\0\0"/0x50450000
    magic: U32<BigEndian>,
    /// Common Object File Format header
    object: COFFHeader,
}

impl PEHeader {
    fn load(input: &[u8], offset: usize) -> Option<&Self> {
        // Check if we have enough bytes to create a header
        if (input.len() - offset) < core::mem::size_of::<Self>() {
            return None;
        }

        // If we do, we can infallibly unwrap since we're Unaligned
        let header = Self::ref_from_bytes(&input[offset..]).unwrap();
        match header.magic.get() {
            0x5045_0000 => Some(header),
            _ => None,
        }
    }
}

#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
struct COFFHeader {
    machine: U16<LittleEndian>,
    section_count: U16<LittleEndian>,
    timestamp: U32<LittleEndian>,
    symbol_offset: U32<LittleEndian>,
    symbol_count: U32<LittleEndian>,
    optional_size: U16<LittleEndian>,
    attributes: U16<LittleEndian>,
}

#[derive(TryFromBytes, KnownLayout, Immutable)]
#[allow(dead_code, clippy::upper_case_acronyms)]
#[repr(u16)]
enum MachineType {
    /// The content of this field is assumed to be applicable to any machine type
    UNKNOWN = 0x0,
    /// Alpha AXP, 32-bit address space
    ALPHA = 0x184,
    /// Alpha AXP 64, 64-bit address space
    ALPHA64 = 0x284,
    /// Matsushita AM33
    AM33 = 0x1D3,
    /// x64
    AMD64 = 0x8664,
    /// ARM little endian
    ARM = 0x1C0,
    /// ARM64 little endian
    ARM64 = 0xAA64,
    /// ARM Thumb-2 little endian
    ARMNT = 0x1C4,
    /// EFI byte code
    EBC = 0xEBC,
    /// Intel 386 or later processors and compatible processors
    I386 = 0x14C,
    /// Intel Itanium processor family
    IA64 = 0x200,
    /// LoongArch 32-bit processor family
    LOONGARCH32 = 0x6232,
    /// LoongArch 64-bit processor family
    LOONGARCH64 = 0x6264,
    /// Mitsubishi M32R little endian
    M32R = 0x9041,
    /// MIPS16
    MIPS16 = 0x266,
    /// MIPS with FPU
    MIPSFPU = 0x366,
    /// MIPS16 with FPU
    MIPSFPU16 = 0x466,
    /// Power PC little endian
    POWERPC = 0x1F0,
    /// Power PC with floating point support
    POWERPCFP = 0x1F1,
    /// MIPS little endian
    R4000 = 0x166,
    /// RISC-V 32-bit address space
    RISCV32 = 0x5032,
    /// RISC-V 64-bit address space
    RISCV64 = 0x5064,
    /// RISC-V 128-bit address space
    RISCV128 = 0x5128,
    /// Hitachi SH3
    SH3 = 0x1A2,
    /// Hitachi SH3 DSP
    SH3DSP = 0x1A3,
    /// Hitachi SH4
    SH4 = 0x1A6,
    /// Hitachi SH5
    SH5 = 0x1A8,
    /// Thumb
    THUMB = 0x1C2,
    /// MIPS little-endian WCE v2
    WCEMIPSV2 = 0x169,
}

#[derive(FromBytes, KnownLayout, Immutable)]
#[repr(transparent)]
pub struct Attributes(u16);

bitflags! {
    impl Attributes: u16 {
        /// Image only, Windows CE, and Microsoft Windows NT and later. This indicates that the file does not contain base relocations and must therefore be loaded at its preferred base address. If the base address is not available, the loader reports an error. The default behavior of the linker is to strip base relocations from executable (EXE) files.
        const RelocsStripped = 1 << 0;
        /// Image only. This indicates that the image file is valid and can be run. If this flag is not set, it indicates a linker error.
        const ExecutableImage = 1 << 1;
        /// COFF line numbers have been removed. This flag is deprecated and should be zero.
        const LineNumsStripped = 1 << 2;
        /// COFF symbol table entries for local symbols have been removed. This flag is deprecated and should be zero.
        const LocalSymsStripped = 1 << 3;
        /// Obsolete. Aggressively trim working set. This flag is deprecated for Windows 2000 and later and must be zero.
        const AggressiveWsTrim = 1 << 4;
        /// Application can handle > 2-GB addresses.
        const LargeAddressAware = 1 << 5;
        /// This flag is reserved for future use.
        const Reserved = 1 << 6;
        /// Little endian: the least significant bit (LSB) precedes the most significant bit (MSB) in memory. This flag is deprecated and should be zero.
        const BytesReversedLo = 1 << 7;
        /// Machine is based on a 32-bit-word architecture.
        const Machine32Bit = 1 << 8;
        /// Debugging information is removed from the image file.
        const DebugStripped = 1 << 9;
        /// If the image is on removable media, fully load it and copy it to the swap file.
        const RemovableRunFromSwap = 1 << 10;
        /// If the image is on network media, fully load it and copy it to the swap file.
        const NetRunFromSwap = 1 << 11;
        /// The image file is a system file, not a user program.
        const System = 1 << 12;
        /// The image file is a dynamic-link library (DLL). Such files are considered executable files for almost all purposes, although they cannot be directly run.
        const Dll = 1 << 13;
        /// The file should be run only on a uniprocessor machine.
        const UpSystemOnly = 1 << 14;
        /// Big endian: the MSB precedes the LSB in memory. This flag is deprecated and should be zero.
        const BytesReversedHi = 1 << 15;
    }
}

#[allow(dead_code)]
impl COFFHeader {
    fn load(input: &[u8], offset: usize) -> Option<&Self> {
        // Check if we have enough bytes to create a header
        if (input.len() - offset) < core::mem::size_of::<Self>() {
            return None;
        }

        // If we do, we can infallibly unwrap since we're Unaligned
        Some(Self::ref_from_bytes(&input[offset..]).unwrap())
    }

    fn machine_type(&self) -> Option<MachineType> {
        zerocopy::try_transmute!(self.machine.get()).ok()
    }

    fn attributes(&self) -> Attributes {
        zerocopy::transmute!(self.attributes.get())
    }
}

#[allow(dead_code)]
enum OptionalHeader<'a> {
    Header32(&'a OptionalHeader32),
    Header64(&'a OptionalHeader64),
}

#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
struct NTHeader32 {
    /// Preferred loading address, must be aligned to 0x10000. Windows CE defaults to 0x10000, DLLs default
    /// to 0x10000000, and modern Windows defaults to 0x400000.
    image_base: U32<LittleEndian>,
    /// Section alignment, must be equal or greater than the file alignment. The default is the page size for
    /// an architecture.
    section_alignment: U32<LittleEndian>,
    /// File alignment, should be a power of 2 between 0x200 and 0x10000. The default is 0x200. If the
    /// section alignment is less than the architecture's page size, this must match the section
    /// alignment.
    file_alignment: U32<LittleEndian>,
    /// Required version for the Operating System
    os_version: Version,
    /// Required version of the image.
    image_version: Version,
    /// Required version of the subsystem.
    subsystem_version: Version,
    /// Reserved, must be zero.
    win32_version: Version,
    /// Total size of the loaded image, including all headers. Must be a multiple of the section alignment.
    image_size: U32<LittleEndian>,
    /// Total size of the MZ header, PE header, and section headers. Must be a multiple of the file
    /// alignment.
    header_size: U32<LittleEndian>,

    checksum: U32<LittleEndian>,
    subsystem: U16<LittleEndian>,
}

#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
struct OptionalHeader32 {
    magic: U16<LittleEndian>,
    linker_version: [u8; 2],
    total_code_size: U32<LittleEndian>,
    total_data_size: U32<LittleEndian>,
    total_bss_size: U32<LittleEndian>,
    entry_point_addr: U32<LittleEndian>,
}

impl OptionalHeader32 {
    fn load(input: &[u8], offset: usize) -> Option<&Self> {
        // Check if we have enough bytes to create a header
        if (input.len() - offset) < core::mem::size_of::<Self>() {
            return None;
        }

        // If we do, we can infallibly unwrap since we're Unaligned
        Some(Self::ref_from_bytes(&input[offset..]).unwrap())
    }
}

#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
struct OptionalHeader64 {}

impl OptionalHeader64 {
    fn load(_input: &[u8], _offset: usize) -> Option<&Self> {
        Some(&OptionalHeader64 {})
    }
}

#[allow(dead_code)]
pub struct PortableExecutable<'a> {
    mz_header: &'a MZHeader,
    pe_header: &'a PEHeader,
    optional_header: Option<OptionalHeader<'a>>,
}

impl<'a> PortableExecutable<'a> {
    #[must_use]
    pub fn new(input: &'a [u8]) -> Option<Self> {
        let mut offset = 0;

        let mz_header = MZHeader::load(input, offset)?;
        offset = mz_header.e_lfanew.get() as usize;

        let pe_header = PEHeader::load(input, offset)?;
        offset += core::mem::size_of::<PEHeader>();

        // Check if we have an optional header
        let optional_header = match pe_header.object.optional_size.get() > 0 {
            true => {
                // If we do, check the magic to see if it's PE32 or PE32+ (if the magic is unknown, just
                // return)
                match u16::from_le_bytes([input[offset], input[offset + 1]]) {
                    //TODO: 0x107?
                    0x10B => Some(OptionalHeader::Header32(OptionalHeader32::load(input, offset)?)),
                    0x20B => Some(OptionalHeader::Header64(OptionalHeader64::load(input, offset)?)),
                    _ => return None,
                }
            }
            false => None,
        };

        Some(Self { mz_header, pe_header, optional_header })
    }
}
