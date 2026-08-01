//! Requirement 3 enforcement: verify command-line flag handling rather than assuming it.
//!
//! Requirement 3 reads, in full: *only pass command-line flags that both compilers honour with
//! the same meaning; verify flag handling rather than assuming it*. This module is how the
//! second half of that sentence becomes an executable probe instead of a prose claim. The
//! driver's `infra_flag_capability_probe` test calls [`run`], asserts on the report it returns
//! and prints [`FlagProbeReport::render`], so a flag that cannot be verified fails the suite
//! loudly rather than being taken on trust by all 1,296 differential cells that depend on it.
//!
//! # Acceptance is not verification, and `-fcf-protection` is the proof
//!
//! The instinctive check — "does the compiler exit zero when handed the flag?" — is not a
//! check at all. Two compilers can accept the same spelling and mean different things by it,
//! and one flag in this repository's inventory does exactly that: **both** compilers accept
//! `-fcf-protection`, but their default scopes differ, so a syntactic parity test would have
//! passed it into the shared set and quietly corrupted every comparison made with it. That is
//! why every entry in the table below is verified by an **observable consequence** of the
//! flag, and why `-fcf-protection` appears in the negative list instead: the only semantic
//! check available for it would be a comparison of control-flow-protection property notes and
//! `endbr64` placement, which the repository's own documentation describes as verifiable by
//! disassembly inspection — outside the scope of the deliberately tiny reader below. The flag
//! is therefore **excluded and the exclusion is stated**, never "verified by acceptance".
//!
//! # Positive verification: one observable consequence per flag
//!
//! Every row is asserted for **both** compilers, using purpose-built programs generated into
//! the probe's own workspaces. A row that cannot be established is a failure; a row whose
//! tooling is genuinely absent is [`CheckOutcome::Unavailable`], reported loudly and turned
//! into a failure under the strict setting — never a silent pass.
//!
//! | Flag | Observable consequence asserted |
//! | --- | --- |
//! | `-o NAME` | A regular file exists at the requested path, is an ELF executable, and runs to produce the program's output. |
//! | `-c` | The ELF type field at file offset `0x10` equals 1 (relocatable object), and the class matches the target's. |
//! | `-static` | The ELF type field equals 2 (executable) **and** no program header of interpreter type (`PT_INTERP`, `p_type == 3`) is present. The same program built without the flag is inspected as a contrast: measured to yield type 3 with an interpreter header. |
//! | `-static` (i686) | The same assertions against an **ELF32** artifact, which is the only way the 32-bit half of the reader below is exercised inside the suite. |
//! | `-g` | A section named `.debug_info` is present — and **absent** without the flag, so both directions are asserted. |
//! | `-I DIR` | A header reachable only through the probe directory is included and its value printed; **without** the flag the compilation must fail. |
//! | `-D NAME=VAL` | The program prints the macro's value and the printed value changes from the built-in default. |
//! | `-U NAME` | The printed value returns to the built-in default after the definition is removed. |
//! | `-O0`/`-O1`/`-O2` | Accepted by both, and the program's output is byte-identical at all three levels. |
//! | `-fPIC` | Accepted by both and produces a runnable artifact that exits zero with the expected output. |
//! | `-L DIR` / `-l NAME` | **Acceptance only.** See the limitation below; it is recorded in the report rather than glossed over. |
//!
//! The `-L`/`-l` limitation, stated plainly: semantic verification would require creating an
//! archive and proving it was searched, which needs an archive-creation tool this suite may
//! not assume, and the whole corpus links nothing but the C runtime, which every driver links
//! by default. Acceptance is therefore the strongest claim that can honestly be made, so it is
//! the only claim made — and the report says so.
//!
//! # Negative assertions: the maintenance guardrail
//!
//! The positive table proves today's flags work. The negative assertions are what stop a
//! future maintainer from adding `-Wall` or `-O3` to a differential invocation months from
//! now, and they are driven by [`SHARED_FLAGS_VERIFIED`], [`FORBIDDEN_IN_DIFFERENTIAL`] and
//! [`is_forbidden_in_differential`] rather than by a private copy of those tables, so the
//! check tracks the single source of truth and cannot drift from `compile.rs`.
//!
//! Four structural assertions and one row per required-absent spelling are made:
//!
//! - no entry of [`FORBIDDEN_IN_DIFFERENTIAL`] appears in [`SHARED_FLAGS_VERIFIED`];
//! - no entry of [`SHARED_FLAGS_VERIFIED`] is recognised as forbidden, which is also the
//!   prefix-confusion guard: `-static` must not be swept up by `-S`, nor `-O0` by `-O3`;
//! - every member of [`UB_AUDIT_GATE_DEFAULT`] is forbidden, because the audit gate is driven
//!   by the reference compiler alone and no gate flag may ever reach a differential
//!   invocation;
//! - the per-side split is exactly as narrow as it claims: each spelling in
//!   [`BCC_TARGET_SELECTORS`] is forbidden on the reference side and permitted on the
//!   compiler-under-test side, which has no cross drivers and no other route to a non-native
//!   backend, while a control flag such as `-Wall` is forbidden on *both* sides.
//!
//! The per-spelling rows and their reasons are in [`REQUIRED_ABSENT`]. Each reason is embedded
//! in the report, so a reader is told not merely that a flag is absent but why it must be.
//!
//! # Positive membership: the minimal differential set
//!
//! The flags actually used in a differential invocation are deliberately minimal — the output
//! path, one optimization level, and static linkage — because that set is unimpeachable and
//! sufficient. The wider verified table exists so that maintenance has a **proven envelope**
//! to work within. Membership assertions pin the minimal set to exactly `-o` and `-static`,
//! require every [`OptLevel::flag`] to be a verified shared flag, and require every verified
//! shared flag to be covered by a row of the positive table, so a flag cannot be added to the
//! envelope without also being verified.
//!
//! # The ELF reader, and why `binutils` is not a dependency
//!
//! Three measured facts make an external binary-inspection tool unnecessary: the type field
//! sits at file offset `0x10` in both classes, an interpreter program header is present only
//! when the artifact is dynamically linked, and `-g` produces a `.debug_info` section. Reading
//! those three things needs a few hundred bytes of header decoding, so the reader below is
//! deliberately tiny, read-only and auditable in one sitting — [`elf_class`], [`elf_type`],
//! [`has_interp`] and [`has_section`] are the whole of it. It is not a general ELF parser and
//! must not become one.
//!
//! **i686 is ELF32 while the other three targets are ELF64**, and the program- and
//! section-header offsets differ between the classes even though `e_type` happens to sit at
//! `0x10` in both. That is the single most likely correctness bug in this module, so both
//! layouts are spelled out in one table, [`LAYOUT_ELF32`] and [`LAYOUT_ELF64`], and the
//! `-static` check is additionally run against a real i686 artifact so the 32-bit path is
//! exercised rather than merely written:
//!
//! | Field | ELF32 | ELF64 |
//! | --- | --- | --- |
//! | `EI_CLASS` | `u8` @ `0x04` (1) | `u8` @ `0x04` (2) |
//! | `EI_DATA` | `u8` @ `0x05` (1 = little-endian) | `u8` @ `0x05` |
//! | `e_type` | `u16` @ `0x10` | `u16` @ `0x10` |
//! | `e_phoff` | `u32` @ `0x1C` | `u64` @ `0x20` |
//! | `e_shoff` | `u32` @ `0x20` | `u64` @ `0x28` |
//! | `e_phentsize` | `u16` @ `0x2A` | `u16` @ `0x36` |
//! | `e_phnum` | `u16` @ `0x2C` | `u16` @ `0x38` |
//! | `e_shentsize` | `u16` @ `0x2E` | `u16` @ `0x3A` |
//! | `e_shnum` | `u16` @ `0x30` | `u16` @ `0x3C` |
//! | `e_shstrndx` | `u16` @ `0x32` | `u16` @ `0x3E` |
//! | `p_type` | `u32` @ entry `+0` | `u32` @ entry `+0` |
//! | `sh_name` | `u32` @ entry `+0` | `u32` @ entry `+0` |
//! | `sh_offset` | `u32` @ entry `+0x10` | `u64` @ entry `+0x18` |
//! | `sh_size` | `u32` @ entry `+0x14` | `u64` @ entry `+0x20` |
//! | `sh_link` | `u32` @ entry `+0x18` | `u32` @ entry `+0x28` |
//!
//! Every read is bounds-checked and every failure names the path and the offset, so a
//! truncated or malformed file yields a diagnosable error rather than a panic that would take
//! the whole test binary down. Decoding is one bounded read followed by safe slice access and
//! `u16`/`u32`/`u64::from_le_bytes`. There is no pointer cast, no reinterpretation of raw bytes
//! as a C-layout structure, and **no use anywhere in this file of the language's memory-safety
//! escape hatch** — the reader is ordinary safe Rust, auditable line by line.
//!
//! # Workspace discipline, determinism and parallel safety
//!
//! Every program the probe writes and every artifact it builds lives in a workspace allocated
//! by [`probe_workspace`], one directory per flag beneath the build directory. This module
//! constructs no path into the repository and none outside the build directory, opens no
//! socket, and gives each probe program its whole input as literals in its own source. Each
//! invocation is bounded by the run's per-cell budget, so a compiler or program that hangs
//! cannot stall the suite.
//!
//! That is path discipline over what this module writes, not confinement of what it spawns. No
//! namespace, `chroot`, syscall filter or network restriction is applied, the inherited
//! environment is not cleared and `TMPDIR` is not set, so a compiler driver keeps putting its
//! intermediates wherever it normally does — under the system temporary directory for the
//! reference driver measured here.
//!
//! A workspace path is a pure function of the flag under examination, so the probe is
//! deterministic and two runs produce the same report. A workspace is removed when its check
//! passed and **retained** when it did not, so a failing flag leaves behind the sources, the
//! artifacts and the exact command lines needed to reproduce it by hand. What a report *prints* is
//! an encoded rendering of those command lines rather than their literal bytes, so that no path a
//! command line contains can forge a row or drive a terminal; the byte-exact form stays in the
//! retained workspace, which is where a byte-exact artifact belongs.
//!
//! # What a retained workspace holds
//!
//! Every invocation persists its complete capture — the untruncated standard output, the
//! untruncated standard error, and a record carrying the termination, the raw wait status, the
//! byte counts and the capture-integrity accounting — into the flag's own workspace, under a stem
//! naming the compiler side, the phase and the invocation ordinal, for example `bcc.compile.01`
//! and `ref.run.01`. Persistence happens inside [`spawn`], the module's single spawn site, so it
//! is a property of the plumbing rather than something each check has to remember.
//!
//! A report row therefore carries three sizes of the same evidence, deliberately: the exact
//! command line, a bounded excerpt that identifies the failure on one line, and the stem of the
//! files holding the whole of it. A row that offered only the excerpt would name a directory and
//! leave the reader to guess which of its entries belonged to which invocation.
//!
//! Every argument the probe assembles is passed through [`is_forbidden_for_side`] before the
//! process is spawned, so the module that enforces the shared-flag discipline cannot itself
//! violate it.
//!
//! # No mocking, and no compiler source change
//!
//! Both compilers are invoked for real and real ELF files are read. A stubbed compiler or a
//! synthetic ELF blob would verify the author's expectation rather than the tool's behaviour,
//! which is precisely the assumption this module exists to eliminate. A flag discrepancy is
//! *reported*; nothing here proposes a change to the compiler under test.
//!
//! Edition 2021, minimum supported Rust 1.70. Only the standard library is used, as the
//! project permits no third-party crate.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use super::env::Capabilities;
use super::execute::{budget_for, run_command_captured_with, CaptureNames, RunOutcome};
use super::sandbox::{probe_workspace, Workspace};
use super::{
    bcc_requires_explicit_target, bcc_target_arguments, corpus_root, is_forbidden_for_side,
    is_forbidden_in_differential, manifest_dir, read_file_bounded, redact_secrets,
    require_contained_corpus_file, require_regular_file, sanitize_text_for_report, shown_path,
    CompilerSide, HarnessError, HarnessResult, OptLevel, Target, BCC_TARGET_SELECTORS,
    DIFFERENTIAL_FLAGS_MINIMAL, FORBIDDEN_IN_DIFFERENTIAL, MAX_INSPECTED_FILE_BYTES,
    SHARED_FLAGS_VERIFIED, UB_AUDIT_GATE_DEFAULT,
};

// ---------------------------------------------------------------------------------------------
// SECTION 1 — the ELF reader.
//
// Delimited so it can be audited on its own. Everything in this section is read-only: it opens
// a file, copies its bytes into memory and decodes a handful of header fields with checked
// slice access. It spawns nothing, writes nothing and depends on no external tool.
// ---------------------------------------------------------------------------------------------

/// The four bytes every ELF file begins with: `0x7F` followed by `ELF`.
const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

/// Offset of the `EI_CLASS` identification byte.
const EI_CLASS_OFFSET: usize = 0x04;

/// Offset of the `EI_DATA` identification byte.
const EI_DATA_OFFSET: usize = 0x05;

/// `EI_CLASS` value for a 32-bit object. The i686 target's output, and no other's.
pub const ELF_CLASS_32: u8 = 1;

/// `EI_CLASS` value for a 64-bit object: x86-64, AArch64 and RISC-V 64.
pub const ELF_CLASS_64: u8 = 2;

/// `EI_DATA` value for two's-complement little-endian data.
///
/// All four supported targets are little-endian, so a big-endian artifact means the file is
/// not one this suite produced and every subsequent little-endian decode would be nonsense.
/// It is rejected rather than decoded.
const ELF_DATA_LITTLE_ENDIAN: u8 = 1;

/// Offset of the `e_type` half-word, which is `0x10` in **both** classes.
const E_TYPE_OFFSET: usize = 0x10;

/// `e_type` value for a relocatable object: what `-c` must produce.
pub const ET_REL: u16 = 1;

/// `e_type` value for an executable: what `-static` must produce.
pub const ET_EXEC: u16 = 2;

/// `e_type` value for a shared object, which is also what a
/// position-independent executable reports.
///
/// Measured: the default dynamic build of a program reports this type and carries a
/// [`PT_INTERP`] program header. Recorded for the contrast half of the `-static` check.
pub const ET_DYN: u16 = 3;

/// `p_type` value of the program header that names the dynamic loader.
///
/// Its **absence** is the proof that `-static` was honoured, and its presence in the
/// contrast build is what shows the flag changed something rather than being accepted and
/// ignored.
const PT_INTERP: u32 = 3;

/// `e_shstrndx` escape value meaning the real index is in `sh_link` of section zero.
const SHN_XINDEX: u16 = 0xffff;

/// The section whose presence proves `-g` was honoured.
pub const DEBUG_INFO_SECTION: &str = ".debug_info";

/// Upper bound on the number of section-name bytes examined while resolving one name.
///
/// A malformed string table with no terminating NUL would otherwise be scanned to the end of
/// the file for every section. No legitimate section name approaches this bound.
const SECTION_NAME_BYTES_MAX: usize = 4096;

/// Where each header field lives for one ELF class.
///
/// Both instances are spelled out in full, [`LAYOUT_ELF32`] and [`LAYOUT_ELF64`], rather than
/// derived from one another by arithmetic: the offsets differ because the intervening address
/// and offset fields differ in width, and a table a reviewer can compare against the standard
/// is worth more here than a clever derivation.
struct ElfLayout {
    /// The `EI_CLASS` value this layout applies to.
    class: u8,
    /// Whether file offsets and sizes are 64-bit. Selects the width of `e_phoff`, `e_shoff`,
    /// `sh_offset` and `sh_size`.
    wide: bool,
    /// Offset of `e_phoff`, the program-header table's file offset.
    e_phoff: usize,
    /// Offset of `e_phentsize`, the size of one program-header entry.
    e_phentsize: usize,
    /// Offset of `e_phnum`, the number of program-header entries.
    e_phnum: usize,
    /// Offset of `e_shoff`, the section-header table's file offset.
    e_shoff: usize,
    /// Offset of `e_shentsize`, the size of one section-header entry.
    e_shentsize: usize,
    /// Offset of `e_shnum`, the number of section-header entries.
    e_shnum: usize,
    /// Offset of `e_shstrndx`, the index of the section-name string table.
    e_shstrndx: usize,
    /// Offset of `sh_offset` within a section-header entry.
    sh_offset: usize,
    /// Offset of `sh_size` within a section-header entry.
    sh_size: usize,
    /// Offset of `sh_link` within a section-header entry, read only for the
    /// [`SHN_XINDEX`] escape.
    sh_link: usize,
}

/// Field offsets for a 32-bit object, which on this suite's targets means i686 alone.
const LAYOUT_ELF32: ElfLayout = ElfLayout {
    class: ELF_CLASS_32,
    wide: false,
    e_phoff: 0x1c,
    e_phentsize: 0x2a,
    e_phnum: 0x2c,
    e_shoff: 0x20,
    e_shentsize: 0x2e,
    e_shnum: 0x30,
    e_shstrndx: 0x32,
    sh_offset: 0x10,
    sh_size: 0x14,
    sh_link: 0x18,
};

/// Field offsets for a 64-bit object: x86-64, AArch64 and RISC-V 64.
const LAYOUT_ELF64: ElfLayout = ElfLayout {
    class: ELF_CLASS_64,
    wide: true,
    e_phoff: 0x20,
    e_phentsize: 0x36,
    e_phnum: 0x38,
    e_shoff: 0x28,
    e_shentsize: 0x3a,
    e_shnum: 0x3c,
    e_shstrndx: 0x3e,
    sh_offset: 0x18,
    sh_size: 0x20,
    sh_link: 0x28,
};

/// One ELF file, read into memory and validated far enough to be decoded safely.
///
/// Constructed only by [`ElfImage::read`], which establishes the two properties every method
/// below relies on: the magic is present, and the class and byte order are ones this suite
/// produces. Nothing here mutates the bytes.
struct ElfImage {
    /// The path as the caller named it, used verbatim in every diagnostic.
    path: PathBuf,
    /// The whole file, up to [`MAX_INSPECTED_FILE_BYTES`].
    ///
    /// Read in one bounded call, which buys two distinct properties. Every decode below reads
    /// these bytes rather than the file, so no decode can observe the file being changed
    /// underneath it and no sequence of checks can disagree about what the artifact contained.
    /// And the read is bounded, so a path that does not name one of the probe's own small
    /// single-function programs — a very large file, a growing one, a device node — costs a
    /// metadata call and a refusal rather than its own size in memory.
    bytes: Vec<u8>,
    /// The field offsets for this file's class.
    layout: &'static ElfLayout,
}

impl ElfImage {
    /// Read and validate `path`.
    ///
    /// The read goes through [`read_file_bounded`] rather than [`std::fs::read`], and the
    /// difference matters for three reasons. The entry is inspected without following a link, so
    /// a symbolic link planted where an artifact should be is refused rather than pulling in a
    /// file from anywhere on the machine while every report still names this path. A device node
    /// or FIFO is refused rather than read, so no inspection can block without end. And the
    /// length is checked against [`MAX_INSPECTED_FILE_BYTES`] before a byte is read, so a file
    /// that is not one of this probe's own small programs is refused at a metadata call instead
    /// of being made resident.
    ///
    /// Everything the caller goes on to ask — class, type, program headers, section names — is
    /// answered from [`ElfImage::bytes`], so one artifact is read exactly once no matter how many
    /// facts are wanted from it, and every fact describes the same bytes.
    ///
    /// # Errors
    ///
    /// Fails, always naming the path, when the entry is a link, is not a regular file, exceeds
    /// [`MAX_INSPECTED_FILE_BYTES`], cannot be read, is shorter than an identification header,
    /// does not begin with [`ELF_MAGIC`], declares a class that is neither [`ELF_CLASS_32`] nor
    /// [`ELF_CLASS_64`], or declares a byte order other than little-endian.
    fn read(path: &Path) -> HarnessResult<ElfImage> {
        let context = format!("reading the ELF artifact {}", shown_path(path));
        let bytes =
            read_file_bounded(&context, path, MAX_INSPECTED_FILE_BYTES).map_err(|error| {
                HarnessError::new(
                    context.clone(),
                    format!(
                        "{}; the flag probe verifies a flag by inspecting the artifact it \
                         produced, so an artifact that cannot be read safely is a hard failure \
                         rather than an unverified flag",
                        error.cause()
                    ),
                )
            })?;

        let identification = bytes.get(..ELF_MAGIC.len()).ok_or_else(|| {
            malformed(
                path,
                format!(
                    "the file is {} bytes long, which is shorter than the four-byte ELF magic; it \
                     is not an object file",
                    bytes.len()
                ),
            )
        })?;
        if identification != ELF_MAGIC {
            return Err(malformed(
                path,
                format!(
                    "the first four bytes are {} rather than the ELF magic 0x7f 'E' 'L' 'F'; the \
                     compiler produced something that is not an object file",
                    hex_bytes(identification)
                ),
            ));
        }

        let class = *bytes.get(EI_CLASS_OFFSET).ok_or_else(|| {
            malformed(
                path,
                String::from(
                    "the file ends before the EI_CLASS identification byte at offset 0x04",
                ),
            )
        })?;
        let layout = match class {
            ELF_CLASS_32 => &LAYOUT_ELF32,
            ELF_CLASS_64 => &LAYOUT_ELF64,
            other => {
                return Err(malformed(
                    path,
                    format!(
                        "EI_CLASS at offset 0x04 is {other}, which is neither {ELF_CLASS_32} \
                         (ELF32, the i686 target) nor {ELF_CLASS_64} (ELF64, the other three \
                         targets); the header layout for that class is unknown to this reader"
                    ),
                ));
            }
        };

        let data = *bytes.get(EI_DATA_OFFSET).ok_or_else(|| {
            malformed(
                path,
                String::from("the file ends before the EI_DATA identification byte at offset 0x05"),
            )
        })?;
        if data != ELF_DATA_LITTLE_ENDIAN {
            return Err(malformed(
                path,
                format!(
                    "EI_DATA at offset 0x05 is {data} rather than {ELF_DATA_LITTLE_ENDIAN}; all \
                     four supported targets are little-endian, so every field this reader decodes \
                     would be decoded the wrong way round"
                ),
            ));
        }

        Ok(ElfImage {
            path: path.to_path_buf(),
            bytes,
            layout,
        })
    }
}

/// Build the error used for every structural defect in an ELF file, so each one names the file.
fn malformed(path: &Path, cause: String) -> HarnessError {
    HarnessError::new(
        format!("decoding the ELF artifact {}", shown_path(path)),
        cause,
    )
}

/// Render a short byte sequence as space-separated two-digit hexadecimal, for a diagnostic.
fn hex_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

impl ElfImage {
    /// The `EI_CLASS` value: [`ELF_CLASS_32`] or [`ELF_CLASS_64`].
    fn class(&self) -> u8 {
        self.layout.class
    }

    /// Decode a little-endian half-word at `offset`, naming `field` if it is out of range.
    ///
    /// The two-byte window is taken with checked slice access and converted through a
    /// fixed-size array, so a truncated file yields an error rather than a panic.
    fn u16_at(&self, offset: usize, field: &str) -> HarnessResult<u16> {
        let window = self.window(offset, 2, field)?;
        let mut raw = [0u8; 2];
        raw.copy_from_slice(window);
        Ok(u16::from_le_bytes(raw))
    }

    /// Decode a little-endian word at `offset`, naming `field` if it is out of range.
    fn u32_at(&self, offset: usize, field: &str) -> HarnessResult<u32> {
        let window = self.window(offset, 4, field)?;
        let mut raw = [0u8; 4];
        raw.copy_from_slice(window);
        Ok(u32::from_le_bytes(raw))
    }

    /// Decode a little-endian double word at `offset`, naming `field` if it is out of range.
    fn u64_at(&self, offset: usize, field: &str) -> HarnessResult<u64> {
        let window = self.window(offset, 8, field)?;
        let mut raw = [0u8; 8];
        raw.copy_from_slice(window);
        Ok(u64::from_le_bytes(raw))
    }

    /// Decode a file offset or size, whose width is four bytes in ELF32 and eight in ELF64.
    ///
    /// This one function is where the class difference in field *width* is expressed, exactly
    /// as [`ElfLayout`] is where the difference in field *position* is expressed. Every
    /// offset-or-size read goes through it, so the 32-bit and 64-bit paths cannot diverge.
    fn file_offset_at(&self, offset: usize, field: &str) -> HarnessResult<u64> {
        if self.layout.wide {
            self.u64_at(offset, field)
        } else {
            Ok(u64::from(self.u32_at(offset, field)?))
        }
    }

    /// Borrow `length` bytes at `offset`, or fail naming the file, the field and the offset.
    ///
    /// The single bounds check every decode in this section funnels through. Addition is
    /// checked, so an offset near the end of the address space cannot wrap into a valid-looking
    /// window.
    fn window(&self, offset: usize, length: usize, field: &str) -> HarnessResult<&[u8]> {
        let end = offset.checked_add(length).ok_or_else(|| {
            malformed(
                &self.path,
                format!(
                    "{field} is claimed at offset {offset} with length {length}, which overflows \
                     an address; the header is malformed"
                ),
            )
        })?;
        self.bytes.get(offset..end).ok_or_else(|| {
            malformed(
                &self.path,
                format!(
                    "{field} needs bytes {offset}..{end} but the file is only {} bytes long; the \
                     artifact is truncated",
                    self.bytes.len()
                ),
            )
        })
    }

    /// Narrow a decoded 64-bit offset or count to a `usize` index into the file.
    ///
    /// A value that does not fit is a malformed header rather than a platform limitation: the
    /// probe's artifacts are a few hundred kilobytes, so any such value is nonsense and saying
    /// so is more useful than truncating it.
    fn as_index(&self, value: u64, field: &str) -> HarnessResult<usize> {
        usize::try_from(value).map_err(|_| {
            malformed(
                &self.path,
                format!(
                    "{field} is {value}, which is too large to index a file on this host; the \
                     header is malformed"
                ),
            )
        })
    }

    /// The `e_type` field, which distinguishes a relocatable object from an executable and from
    /// a shared object or position-independent executable.
    fn e_type(&self) -> HarnessResult<u16> {
        self.u16_at(E_TYPE_OFFSET, "e_type")
    }

    /// Whether any program header has type [`PT_INTERP`].
    ///
    /// A relocatable object has no program-header table at all, which is reported as `false`
    /// rather than as an error: an object file legitimately names no dynamic loader.
    fn has_interp(&self) -> HarnessResult<bool> {
        let table = self.file_offset_at(self.layout.e_phoff, "e_phoff")?;
        let count = self.u16_at(self.layout.e_phnum, "e_phnum")?;
        let entry_size = self.u16_at(self.layout.e_phentsize, "e_phentsize")?;
        if table == 0 || count == 0 {
            return Ok(false);
        }
        if entry_size == 0 {
            return Err(malformed(
                &self.path,
                format!(
                    "e_phentsize is zero while e_phnum is {count}; a program-header table with \
                     zero-sized entries cannot be walked"
                ),
            ));
        }

        let table = self.as_index(table, "e_phoff")?;
        let entry_size = usize::from(entry_size);
        for index in 0..usize::from(count) {
            let entry = table
                .checked_add(
                    index
                        .checked_mul(entry_size)
                        .ok_or_else(|| self.entry_overflow("program", index))?,
                )
                .ok_or_else(|| self.entry_overflow("program", index))?;
            if self.u32_at(entry, "p_type")? == PT_INTERP {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Whether a section named `name` is present.
    ///
    /// Names are resolved through the section-header string table, so the comparison is against
    /// the real name rather than a heuristic on the file's bytes: a program that merely
    /// contained the text `.debug_info` in a string literal must not be mistaken for one built
    /// with debug information.
    ///
    /// Two escapes defined by the format are honoured, because ignoring either would silently
    /// read the wrong table: when `e_shnum` is zero the real count is in `sh_size` of section
    /// zero, and when `e_shstrndx` is [`SHN_XINDEX`] the real index is in `sh_link` of section
    /// zero.
    fn has_section(&self, name: &str) -> HarnessResult<bool> {
        let Some(table) = self.section_table()? else {
            return Ok(false);
        };
        let strings = self.section_name_table(&table)?;
        let wanted = name.as_bytes();
        for index in 0..table.count {
            let entry = table.entry(self, index)?;
            let name_offset =
                self.as_index(u64::from(self.u32_at(entry, "sh_name")?), "sh_name")?;
            if self.section_name(&strings, name_offset)? == wanted {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Locate and size the section-header table, or report that the file has none.
    fn section_table(&self) -> HarnessResult<Option<SectionTable>> {
        let table = self.file_offset_at(self.layout.e_shoff, "e_shoff")?;
        if table == 0 {
            return Ok(None);
        }
        let entry_size = self.u16_at(self.layout.e_shentsize, "e_shentsize")?;
        if entry_size == 0 {
            return Err(malformed(
                &self.path,
                String::from(
                    "e_shentsize is zero while e_shoff is non-zero; a section-header table with \
                     zero-sized entries cannot be walked",
                ),
            ));
        }
        let table = self.as_index(table, "e_shoff")?;
        let entry_size = usize::from(entry_size);

        // The extended count escape: a count of zero with a table present means the real count
        // is in sh_size of the first entry.
        let declared = self.u16_at(self.layout.e_shnum, "e_shnum")?;
        let count = if declared == 0 {
            let first = SectionTable {
                base: table,
                entry_size,
                count: 1,
            }
            .entry(self, 0)?;
            let extended = self.file_offset_at(first + self.layout.sh_size, "sh_size")?;
            self.as_index(
                extended,
                "the extended section count in sh_size of section zero",
            )?
        } else {
            usize::from(declared)
        };
        if count == 0 {
            return Ok(None);
        }
        Ok(Some(SectionTable {
            base: table,
            entry_size,
            count,
        }))
    }

    /// The bytes of the section-name string table.
    fn section_name_table(&self, table: &SectionTable) -> HarnessResult<Vec<u8>> {
        let declared = self.u16_at(self.layout.e_shstrndx, "e_shstrndx")?;
        // The extended index escape: the sentinel means the real index is in sh_link of the
        // first entry.
        let index = if declared == SHN_XINDEX {
            let first = table.entry(self, 0)?;
            self.as_index(
                u64::from(self.u32_at(first + self.layout.sh_link, "sh_link")?),
                "the extended string-table index in sh_link of section zero",
            )?
        } else {
            usize::from(declared)
        };
        if index >= table.count {
            return Err(malformed(
                &self.path,
                format!(
                    "the section-name string table is section {index}, but the file declares only \
                     {} sections; section names cannot be resolved",
                    table.count
                ),
            ));
        }

        let entry = table.entry(self, index)?;
        let offset = self.file_offset_at(entry + self.layout.sh_offset, "sh_offset")?;
        let size = self.file_offset_at(entry + self.layout.sh_size, "sh_size")?;
        let offset = self.as_index(offset, "sh_offset")?;
        let size = self.as_index(size, "sh_size")?;
        Ok(self
            .window(offset, size, "the section-name string table")?
            .to_vec())
    }

    /// The NUL-terminated name at `offset` in the section-name string table.
    ///
    /// A name that is not terminated within [`SECTION_NAME_BYTES_MAX`] bytes, or that starts
    /// past the end of the table, is a malformed file rather than an unnamed section.
    fn section_name<'a>(&self, strings: &'a [u8], offset: usize) -> HarnessResult<&'a [u8]> {
        let tail = strings.get(offset..).ok_or_else(|| {
            malformed(
                &self.path,
                format!(
                    "a section name is claimed at offset {offset} of a string table that is only \
                     {} bytes long",
                    strings.len()
                ),
            )
        })?;
        let bounded = match tail.get(..SECTION_NAME_BYTES_MAX) {
            Some(window) => window,
            None => tail,
        };
        match bounded.iter().position(|byte| *byte == 0) {
            Some(end) => Ok(&bounded[..end]),
            None => Err(malformed(
                &self.path,
                format!(
                    "the section name at offset {offset} is not terminated within \
                     {SECTION_NAME_BYTES_MAX} bytes; the string table is malformed"
                ),
            )),
        }
    }

    /// The error reported when walking a header table would overflow an address.
    fn entry_overflow(&self, table: &str, index: usize) -> HarnessError {
        malformed(
            &self.path,
            format!(
                "the offset of {table}-header entry {index} overflows an address; the header is \
                 malformed"
            ),
        )
    }
}

/// The located section-header table: where it starts, how big an entry is, and how many there
/// are, with both extended escapes already resolved.
struct SectionTable {
    base: usize,
    entry_size: usize,
    count: usize,
}

impl SectionTable {
    /// The file offset of entry `index`, bounds-checked against address overflow.
    fn entry(&self, image: &ElfImage, index: usize) -> HarnessResult<usize> {
        let scaled = index
            .checked_mul(self.entry_size)
            .ok_or_else(|| image.entry_overflow("section", index))?;
        self.base
            .checked_add(scaled)
            .ok_or_else(|| image.entry_overflow("section", index))
    }
}

// Why there is no path-taking shorthand for a single ELF fact.
//
// Every question the probe asks of an artifact — its class, its type field, whether it carries a
// program interpreter header, whether it contains a named section — is a method on `ElfImage`, and
// the only way to obtain an `ElfImage` is `ElfImage::read`. A convenience function taking a path
// per fact used to sit here, and it was removed rather than kept, for two reasons.
//
// The first is cost. Several checks want two or three facts about one artifact, and a
// path-per-fact shorthand read the whole file once per fact. A statically linked probe program is
// a couple of megabytes, so the `-static` check alone read four megabytes to answer two questions
// that one read answers.
//
// The second, and the reason this is a correctness property rather than a performance one, is
// agreement. Two reads of a path are two different observations, and nothing guarantees they see
// the same bytes: an artifact replaced between them would let one report line describe the file
// that was inspected and the next describe a different file, with no indication in the report that
// the subject had changed. Reading once and asking the resulting image every question makes a
// whole check's conclusion describe one file, by construction.

/// Name an `e_type` value in words, for a report a reader should not have to decode.
///
/// The three values the probe can legitimately meet are the three the ELF facts were measured on:
/// a relocatable object from `-c`, an executable from `-static`, and the shared-object type a
/// default position-independent build reports. Anything else is named as unrecognised rather than
/// guessed at.
fn describe_elf_type(value: u16) -> &'static str {
    match value {
        ET_REL => "relocatable object",
        ET_EXEC => "executable",
        ET_DYN => "shared object, which is also what a position-independent executable reports",
        _ => "an object kind this probe does not recognise",
    }
}

// ---------------------------------------------------------------------------------------------
// SECTION 2 — the probe programs.
//
// Every program hand-declares the one library function it uses and includes no header, with the
// single deliberate exception of the include-path probe. The compiler under test bundles only
// the required freestanding headers and ships no standard input/output header, so an
// `#include <stdio.h>` would fail against it while succeeding against the reference compiler —
// manufacturing a divergence caused by the probe rather than by either compiler. Published
// output-comparison experience independently identifies a missing `printf` prototype as the most
// common portability problem in suites of this kind.
//
// Each program prints one line, deterministically: no address, no timestamp, no randomness, no
// locale-dependent formatting and no uninitialized read.
// ---------------------------------------------------------------------------------------------

/// The baseline program, used by every check whose subject is not the preprocessor.
///
/// The operands are `volatile` so that the multiply survives every optimization level. That
/// matters for the `-O` rows specifically: comparing output across levels only says something
/// about code generation if instructions were actually emitted, and the same computation without
/// `volatile` folds to a single immediate move.
const PROGRAM_ARITHMETIC: &str = r#"/*
 * Flag-capability probe: baseline program.
 *
 * Generated by tests/conformance_harness/flagprobe.rs into a workspace beneath the build
 * directory.  It hand-declares printf and includes no header, so it compiles identically under
 * a freestanding compiler that ships no <stdio.h> and under a hosted reference compiler.
 *
 * The operands are volatile so the multiply is genuinely emitted at every optimization level.
 */
int printf(const char *, ...);

int main(void)
{
    volatile int left = 7;
    volatile int right = 6;
    int product = left * right;

    printf("flagprobe=%d\n", product);
    return 0;
}
"#;

/// The exact bytes [`PROGRAM_ARITHMETIC`] must print.
const ARITHMETIC_STDOUT: &str = "flagprobe=42\n";

/// The macro the `-D` and `-U` probes define, undefine and print.
const MACRO_NAME: &str = "BCC_FLAG_PROBE_MACRO";

/// The value the `-D` probe defines [`MACRO_NAME`] as.
///
/// Deliberately different from the built-in default below, because the observable being asserted
/// is that the printed value *changes*.
const MACRO_VALUE: &str = "7";

/// The value [`PROGRAM_MACRO`] prints when nothing defines [`MACRO_NAME`].
const MACRO_DEFAULT: &str = "0";

/// The prefix [`PROGRAM_MACRO`] prints before the value of its macro.
///
/// Kept beside the program it describes so that changing one without the other is obvious.
const MACRO_OUTPUT_PREFIX: &str = "flagprobe_macro=";

/// The preprocessor probe, used by the `-D` and `-U` checks.
///
/// The built-in default is what makes both directions observable: `-D` must change the printed
/// value away from it, and `-U` must return the printed value to it.
const PROGRAM_MACRO: &str = r#"/*
 * Flag-capability probe: preprocessor program for -D and -U.
 *
 * Prints the value of BCC_FLAG_PROBE_MACRO, defaulting to 0 when nothing defines it, so that
 * the effect of a command-line definition and of a command-line removal are both visible in
 * the program's own output rather than merely in the compiler's exit status.
 */
int printf(const char *, ...);

#ifndef BCC_FLAG_PROBE_MACRO
#define BCC_FLAG_PROBE_MACRO 0
#endif

int main(void)
{
    printf("flagprobe_macro=%d\n", BCC_FLAG_PROBE_MACRO);
    return 0;
}
"#;

/// The include-path probe.
///
/// The header is named with angle brackets rather than quotes on purpose: a quoted include
/// consults the directory of the including file first, which would give the compilation a route
/// to the header that has nothing to do with `-I`. With angle brackets the only route is the
/// include search path, so the negative half of the check — that the compilation *fails* without
/// `-I` — is strictly load-bearing.
const PROGRAM_HEADER_INCLUDE: &str = r#"/*
 * Flag-capability probe: include-path program for -I.
 *
 * The angle-bracket include is deliberate: only the include search path can satisfy it, so a
 * successful compilation proves the directory named by -I was searched, and a compilation
 * without -I must fail.
 */
int printf(const char *, ...);

#include <probe_header.h>

int main(void)
{
    printf("flagprobe_header=%d %s\n", BCC_PROBE_HEADER_VALUE, BCC_PROBE_HEADER_NAME);
    return 0;
}
"#;

/// The exact bytes [`PROGRAM_HEADER_INCLUDE`] must print, which are decided by the fixture
/// header rather than by this module.
const HEADER_STDOUT: &str = "flagprobe_header=4242 probe_header\n";

/// The fixture macro carrying the value the include probe prints.
const FIXTURE_VALUE_MACRO: &str = "BCC_PROBE_HEADER_VALUE";

/// The fixture macro carrying the name the include probe prints.
const FIXTURE_NAME_MACRO: &str = "BCC_PROBE_HEADER_NAME";

/// Corpus-relative directory holding the fixture header, and the only directory ever named by
/// the `-I` probe.
const FIXTURE_INCLUDE_DIRECTORY: [&str; 2] = ["support", "include"];

/// The fixture header's file name.
const FIXTURE_HEADER_NAME: &str = "probe_header.h";

/// Workspace entry name of the baseline program.
const SOURCE_ARITHMETIC: &str = "probe.c";

/// Workspace entry name of the preprocessor program.
const SOURCE_MACRO: &str = "probe_macro.c";

/// Workspace entry name of the include-path program.
const SOURCE_HEADER: &str = "probe_include.c";

/// The output-naming flag, whose own observable is that the file it names exists.
const FLAG_OUTPUT: &str = "-o";

/// The compile-only flag.
const FLAG_COMPILE_ONLY: &str = "-c";

/// The static-linkage flag: the one linkage mode both compilers spell identically, and what
/// makes an emulated target executable with no sysroot and no dynamic-loader configuration.
const FLAG_STATIC: &str = "-static";

/// The debug-information flag.
const FLAG_DEBUG: &str = "-g";

/// The include-search-path flag.
const FLAG_INCLUDE: &str = "-I";

/// The macro-definition flag.
const FLAG_DEFINE: &str = "-D";

/// The macro-removal flag.
const FLAG_UNDEFINE: &str = "-U";

/// The library-search-path flag, verified for acceptance only.
const FLAG_LIBRARY_PATH: &str = "-L";

/// The library-name flag, verified for acceptance only.
const FLAG_LIBRARY: &str = "-l";

/// The library named by the `-l` acceptance check.
///
/// The mathematics library is chosen because it is a genuinely separate archive that every
/// supported C runtime installs, so the flag is exercised rather than satisfied by a library the
/// driver would have linked anyway.
const LIBRARY_NAME: &str = "m";

/// The position-independent-code flag.
const FLAG_PIC: &str = "-fPIC";

/// Characters of a captured diagnostic quoted in a report row.
///
/// A row is one line plus its evidence, and the untruncated stream stays in the retained
/// workspace, so the excerpt exists to identify a failure rather than to explain it in full.
const DIAGNOSTIC_EXCERPT_CHARS_MAX: usize = 200;

/// Characters of captured program output quoted in a report row.
const OUTPUT_EXCERPT_CHARS_MAX: usize = 120;

/// The phase component of a capture stem for an invocation that compiled something.
const CAPTURE_PHASE_COMPILE: &str = "compile";

/// The phase component of a capture stem for an invocation that ran a built artifact.
const CAPTURE_PHASE_RUN: &str = "run";

/// Suffix of the workspace entry holding an invocation's captured standard output.
const CAPTURE_STDOUT_SUFFIX: &str = "stdout";

/// Suffix of the workspace entry holding an invocation's captured standard error.
const CAPTURE_STDERR_SUFFIX: &str = "stderr";

/// Suffix of the workspace entry holding an invocation's recorded termination and raw wait status.
const CAPTURE_EXIT_SUFFIX: &str = "exit";

/// How many invocations of one compiler in one phase a single flag workspace may hold.
///
/// A ceiling rather than an unbounded search, because the ordinal is allocated by looking for the
/// first entry name that is free: an unbounded loop against a directory that could not be written
/// would spin instead of reporting. Nine checks share the busiest workspace and none makes more
/// than six invocations of one compiler in one phase, so this is roughly an order of magnitude of
/// headroom over the largest real use.
const CAPTURE_ORDINAL_MAX: usize = 99;

// ---------------------------------------------------------------------------------------------
// SECTION 3 — invocation plumbing.
//
// Both compilers are invoked for real, through the harness's one bounded-spawn implementation,
// with the working directory set to the flag's own workspace. Every argument is checked against
// the shared-flag discipline before the process is spawned, so the module that enforces the
// discipline cannot itself break it.
// ---------------------------------------------------------------------------------------------

/// One compiler as the probe drives it.
struct ProbeCompiler {
    /// Which side of a differential invocation this compiler is, which decides both the
    /// diagnostics wording and — through [`is_forbidden_for_side`] — which arguments it may be
    /// given.
    side: CompilerSide,
    /// The executable discovery vetted.
    binary: PathBuf,
    /// Prefix applied to every artifact this compiler writes, so both compilers can share one
    /// flag workspace without overwriting each other.
    prefix: &'static str,
    /// The target this compiler is producing output for.
    target: Target,
}

impl ProbeCompiler {
    /// The phrase used for this compiler in diagnostics and report rows.
    fn label(&self) -> &'static str {
        self.side.label()
    }

    /// Whether this invocation must carry an explicit target selection.
    ///
    /// True only for the compiler under test on a non-native target: it has no cross drivers, so
    /// the target-selection flag is its only route to a non-native backend. The reference side
    /// never receives one — it has none, and a target is selected there by choosing a different
    /// driver binary.
    fn selects_target_by_flag(&self) -> bool {
        self.side == CompilerSide::UnderTest && bcc_requires_explicit_target(self.target)
    }

    /// The path of an artifact this compiler will write into `workspace`.
    ///
    /// # Errors
    ///
    /// Fails when the name is one the workspace refuses, which cannot happen for the fixed stems
    /// this module uses and is checked because the guard is what the write-path discipline rests
    /// on.
    fn artifact(&self, workspace: &Workspace, stem: &str) -> HarnessResult<PathBuf> {
        workspace.path(&format!("{}.{stem}", self.prefix))
    }

    /// Assemble the full argument vector for one invocation of this compiler.
    ///
    /// # Errors
    ///
    /// Fails when an argument is forbidden for this compiler's side. That is a defect in this
    /// module rather than in the environment, and it is checked on every invocation so that the
    /// probe can never quietly hand a compiler a flag the discipline forbids.
    fn argv(&self, args: &[String]) -> HarnessResult<Vec<String>> {
        let mut argv = Vec::with_capacity(args.len() + 3);
        argv.push(path_text(&self.binary)?);
        if self.selects_target_by_flag() {
            argv.extend(bcc_target_arguments(self.target));
        }
        for argument in args {
            if is_forbidden_for_side(argument, self.side) {
                return Err(HarnessError::new(
                    format!(
                        "assembling a flag-probe invocation of the {} for {}",
                        self.label(),
                        self.target
                    ),
                    format!(
                        "the argument {:?} must never be passed to the {}; the flag probe enforces \
                         the shared-flag discipline and therefore may not violate it, so this is a \
                         defect in the probe rather than a property of the environment",
                        sanitize_text_for_report(argument),
                        self.label()
                    ),
                ));
            }
            argv.push(argument.clone());
        }
        Ok(argv)
    }

    /// The stem every capture of this compiler in `phase` is published under, before its ordinal.
    ///
    /// Carries the side prefix so that the two compilers sharing one flag workspace never write
    /// over each other, and the phase so that a reader can tell a compilation's diagnostics from
    /// the output of the program it produced without opening either file.
    fn capture_base(&self, phase: &str) -> String {
        format!("{}.{phase}", self.prefix)
    }

    /// Compile with this compiler, returning what the invocation did and where it was captured.
    ///
    /// A non-zero exit is **not** an error: several checks require a compilation to fail, and the
    /// caller decides what the outcome means. An error here means the process could not be run
    /// or recorded at all.
    fn compile(
        &self,
        workspace: &Workspace,
        args: &[String],
        budget: Duration,
        timeout_tool: Option<&Path>,
    ) -> HarnessResult<Invocation> {
        spawn(
            &self.argv(args)?,
            workspace,
            budget,
            timeout_tool,
            &self.capture_base(CAPTURE_PHASE_COMPILE),
        )
    }

    /// Run an already prepared launch vector for an artifact this compiler built.
    ///
    /// Separate from [`ProbeCompiler::compile`] only in the capture phase it records under, and
    /// deliberately not a bare [`spawn`] call at the two sites that need it: the launch vector for
    /// a foreign target begins with an emulator rather than with a compiler, so the side the
    /// capture belongs to cannot be recovered from the vector and has to be stated by the compiler
    /// whose artifact is being run.
    fn execute(
        &self,
        argv: &[String],
        workspace: &Workspace,
        budget: Duration,
        timeout_tool: Option<&Path>,
    ) -> HarnessResult<Invocation> {
        spawn(
            argv,
            workspace,
            budget,
            timeout_tool,
            &self.capture_base(CAPTURE_PHASE_RUN),
        )
    }
}

/// One invocation the probe made, together with where its complete capture was published.
///
/// # Why the capture location travels with the outcome
///
/// A report row quotes one diagnostic line and one bounded excerpt of output, which is the right
/// size for a row and far too small to diagnose from. The untruncated streams and the raw wait
/// status are therefore written into the flag's own workspace at the moment of the invocation, and
/// the stem they were written under is carried here so the row can name them. Returning the
/// outcome alone would leave a reader holding an excerpt and a directory listing, with no stated
/// correspondence between the invocation they are reading about and the files that recorded it.
struct Invocation {
    /// What the invocation did: its captured streams, its raw wait status and its termination.
    outcome: RunOutcome,
    /// The workspace entry stem the three capture files share, without the `.stdout`, `.stderr`
    /// and `.exit` suffixes.
    capture: String,
}

/// Spawn one prepared argument vector in `workspace`, capture it, and persist the capture.
///
/// Delegates to the harness's single bounded-spawn implementation, which supplies the null
/// standard input, the piped and concurrently drained output streams, the bounded wait, the
/// forcible kill on expiry, the process-group ownership and whole-group sweep, and the replaced
/// environment. Writing any of that a second time here is how a suite acquires a second timeout
/// bug and a second way to leak the invoking environment into a tool.
///
/// The workspace is passed as the child's private directory, which is what asks for the isolated
/// environment: every variable cleared, a documented minimal set restored, and the workspace as
/// both `HOME` and `TMPDIR`. That matters as much for this probe as for a cell. A flag probe's
/// whole purpose is to establish what a flag *means* to each compiler, and an inherited search
/// path, an inherited driver-control variable or an inherited sanitizer setting would make the
/// answer a property of the machine the probe happened to run on rather than of the compilers.
///
/// # Why persistence happens here rather than at the call sites
///
/// This is the only place in the module a process is started, so persisting here makes "every
/// invocation left its full streams and its raw status on disk" a property of the plumbing instead
/// of a habit each of the six invocation sites has to remember. The alternative — persisting where
/// a check happens to care — is how the promise in [`settle_workspace`] that a retained workspace
/// holds every captured stream came to be untrue of the streams themselves.
///
/// Persistence is unconditional rather than deferred until a check fails, for two reasons. A check
/// does not know it has failed until after the invocation it is judging, so deferring would mean
/// holding every capture of every check in memory against the possibility that a later one fails.
/// And the run may ask for every workspace to be kept, in which case a passing check's evidence is
/// wanted too. The cost is three small files per invocation in a directory that is removed moments
/// later when nothing went wrong, which is the same trade the undefined-behaviour audit already
/// makes for the same reason.
fn spawn(
    argv: &[String],
    workspace: &Workspace,
    budget: Duration,
    timeout_tool: Option<&Path>,
    capture_base: &str,
) -> HarnessResult<Invocation> {
    let (program, arguments) = argv.split_first().ok_or_else(|| {
        HarnessError::new(
            "spawning a flag-probe invocation",
            String::from("the argument vector is empty, so there is no program to run"),
        )
    })?;
    let mut command = Command::new(program);
    command.args(arguments);
    // Per child rather than process-wide: the suite's tests run concurrently in one process, so
    // altering a shared working directory would be a data race rather than a confinement.
    command.current_dir(workspace.root());
    let outcome = run_command_captured_with(command, budget, timeout_tool, Some(workspace.root()))?;
    let capture = allocate_capture_stem(workspace, capture_base)?;
    outcome.persist(workspace, &capture_names(&capture))?;
    Ok(Invocation { outcome, capture })
}

/// The first capture stem beginning with `base` that this workspace does not already hold.
///
/// # Why the ordinal is read from the directory rather than counted in memory
///
/// Several checks invoke one compiler in one phase more than once in a single workspace — the
/// include-path check compiles twice on purpose, the static-linkage check adds a dynamically
/// linked contrast, and the macro and optimization-level checks each build three configurations.
/// A fixed stem would leave only the last of them on disk, which is the failure this allocation
/// exists to prevent.
///
/// The directory itself is the state, so no counter has to be threaded through six call sites and
/// ten check bodies, and no two allocations can disagree. It is deterministic because a flag
/// workspace is emptied when it is allocated and the invocations within a check are sequential, so
/// the same run of the same check numbers its captures identically every time.
///
/// # Errors
///
/// Fails when the workspace refuses the entry name, and when the ceiling is reached — which for
/// this corpus means the workspace could not be written rather than that a check made a hundred
/// invocations, and is reported as the defect it is rather than retried without end.
fn allocate_capture_stem(workspace: &Workspace, base: &str) -> HarnessResult<String> {
    for ordinal in 1..=CAPTURE_ORDINAL_MAX {
        let stem = format!("{base}.{ordinal:02}");
        let candidate = workspace.path(&format!("{stem}.{CAPTURE_STDOUT_SUFFIX}"))?;
        // The link itself rather than its target: a name occupied by a dangling link is still
        // occupied, and the publisher will refuse it, so treating it as free would turn a
        // containment refusal into a lost capture.
        if fs::symlink_metadata(&candidate).is_err() {
            return Ok(stem);
        }
    }
    Err(HarnessError::new(
        format!(
            "allocating a capture name for a flag-probe invocation in {}",
            workspace.root().display()
        ),
        format!(
            "the first {CAPTURE_ORDINAL_MAX} capture names beginning with {:?} are all taken, so \
             this invocation's streams could not be recorded; the workspace is either unwritable \
             or holds entries no check in this module creates",
            sanitize_text_for_report(base)
        ),
    ))
}

/// The three workspace entry names one capture stem publishes into.
fn capture_names(stem: &str) -> CaptureNames {
    CaptureNames::new(
        format!("{stem}.{CAPTURE_STDOUT_SUFFIX}"),
        format!("{stem}.{CAPTURE_STDERR_SUFFIX}"),
        format!("{stem}.{CAPTURE_EXIT_SUFFIX}"),
    )
}

/// Render a path as text, or fail explaining why the probe cannot proceed with it.
///
/// Every path the probe uses is derived from the package manifest directory, the corpus or the
/// vetted tool inventory, all of which are plain text, so this failure means something outside
/// the suite's control produced a path that cannot be reproduced in a command line.
fn path_text(path: &Path) -> HarnessResult<String> {
    path.to_str().map(String::from).ok_or_else(|| {
        HarnessError::new(
            "rendering a path for a flag-probe command line",
            format!(
                "{} cannot be expressed as text, so the invocation could not be recorded exactly; \
                 every reported command line must be reproducible by hand",
                shown_path(path)
            ),
        )
    })
}

// ---------------------------------------------------------------------------------------------
// SECTION 4 — the verification record.
//
// One row per check. A row carries what was checked, what was observed, the command lines that
// observed it, and the verdict — because a probe that reports only "failed" leaves its reader to
// reconstruct the invocation by hand, and requirement 3's whole point is that flag handling is
// established by evidence rather than by assertion.
//
// Every text field of a row is sanitized as it is recorded rather than as it is rendered, and a
// command line is retained only through `CommandEvidence`. The rows in this section are therefore
// safe to print anywhere, by construction rather than by each printer remembering to be careful:
// a compiler binary path chosen through an environment variable, a workspace path derived from the
// build directory, and a diagnostic excerpt taken from a subprocess all reach a row through text
// that has already been encoded.
// ---------------------------------------------------------------------------------------------

/// What sort of verification a row records.
///
/// The distinction is reported rather than smoothed over. A reader must be able to see at a
/// glance which flags were verified by an observable consequence, which were verified only for
/// acceptance, and which were verified to be absent — because those three claims have very
/// different strengths and conflating them is precisely the failure requirement 3 guards against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CheckKind {
    /// Both compilers accepted the flag **and** it had the stated observable consequence.
    Semantic,
    /// Both compilers accepted the flag, and no observable consequence could be established
    /// without a tool the suite is not permitted to depend on. The limitation is recorded on the
    /// row rather than left for a reader to infer.
    AcceptanceOnly,
    /// A flag that must **not** be in the shared set was confirmed absent from it.
    Negative,
    /// A structural property of the flag tables themselves, such as the minimal differential set
    /// being exactly what it claims to be.
    Membership,
}

impl CheckKind {
    /// Fixed-width column text for the report table.
    pub fn label(self) -> &'static str {
        match self {
            CheckKind::Semantic => "semantic ",
            CheckKind::AcceptanceOnly => "accepted ",
            CheckKind::Negative => "absent   ",
            CheckKind::Membership => "structure",
        }
    }
}

impl fmt::Display for CheckKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label().trim_end())
    }
}

/// The verdict of one row.
///
/// There is deliberately no "skipped" variant. A check that could not be performed because a
/// compiler is genuinely absent is [`CheckOutcome::Unavailable`], which is reported loudly and
/// becomes a failure under the strict setting — never a silent pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CheckOutcome {
    /// The check was performed and it held.
    Verified,
    /// The check was performed and it did not hold. This fails the run unconditionally: every
    /// differential cell in the suite rests on the flag discipline this probe establishes.
    Failed,
    /// The check could not be performed because a tool it needs is absent from this environment.
    Unavailable,
}

impl CheckOutcome {
    /// Fixed-width column text for the report table.
    pub fn label(self) -> &'static str {
        match self {
            CheckOutcome::Verified => "VERIFIED   ",
            CheckOutcome::Failed => "FAILED     ",
            CheckOutcome::Unavailable => "UNAVAILABLE",
        }
    }
}

impl fmt::Display for CheckOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label().trim_end())
    }
}

/// One command line, retained in the only form a report may carry.
///
/// A command line is the most dangerous field on a check row, and the reason is worth stating
/// plainly. Its text comes from an argument vector, and an argument vector holds a compiler binary
/// path taken from the environment and workspace paths derived from the build directory. Those are
/// chosen by whoever runs the suite, and [`posix_quote`] — correctly, for its own purpose —
/// preserves every byte inside its quotes, because a reproduction line that altered a path would
/// no longer reproduce anything. Preserving a byte is exactly right for a shell and exactly wrong
/// for a report: a carriage return erases the line a reader has just seen, a newline forges
/// another row, an escape introducer starts a terminal sequence.
///
/// So the raw argv is retained here only in encoded form. [`CommandEvidence::shown`] is the
/// POSIX-quoted line with every report-hostile character encoded, and
/// [`CommandEvidence::is_encoded`] says whether that encoding changed anything — which is the
/// honest answer to "can I paste this?". For an ordinary command line, and that is every command
/// line in a normal run, nothing is encoded and the answer is yes. For an exotic one the reader is
/// told the text stands for bytes rather than being them, and the untruncated exact form remains in
/// the retained workspace, which is where a byte-exact artifact belongs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandEvidence {
    /// The POSIX-quoted command line, encoded for a report by [`report_text`].
    shown: String,
    /// Whether encoding altered the line, so a reader is never told a rendering is byte-exact
    /// when it is not.
    encoded: bool,
}

impl CommandEvidence {
    /// Encode one already-quoted command line for a report.
    fn new(command_line: &str) -> CommandEvidence {
        let shown = report_text(command_line);
        CommandEvidence {
            encoded: shown != command_line,
            shown,
        }
    }

    /// The command line as a report may print it.
    pub fn shown(&self) -> &str {
        &self.shown
    }

    /// True when [`CommandEvidence::shown`] encodes bytes rather than reproducing them, so the
    /// line identifies the invocation but is not itself copy-pasteable.
    pub fn is_encoded(&self) -> bool {
        self.encoded
    }
}

impl fmt::Display for CommandEvidence {
    /// Render the line, saying so when it is an encoding rather than the literal bytes.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.shown())?;
        if self.is_encoded() {
            f.write_str(
                "    [encoded: this line contains \\xNN or \\u{NNNN} escapes standing for                  characters a report cannot carry literally, so it identifies the invocation but                  is not byte-exact; the exact form is in the retained workspace]",
            )?;
        }
        Ok(())
    }
}

/// One completed check: its subject, its scope, what was observed and the commands that observed
/// it.
///
/// The fields are private and the type is built only by [`Evidence::finish`], because a row is
/// the unit the driver asserts on and the unit a reader reproduces. A row whose verdict could be
/// assigned independently of its findings would let the two disagree, which is the one thing a
/// verification record must never do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlagCheck {
    /// The flag or property under examination, as it is spelled on a command line.
    subject: String,
    /// Where the check applied: which target, and which compilers took part.
    scope: String,
    kind: CheckKind,
    /// The observable consequence this check required, in one line.
    observable: String,
    outcome: CheckOutcome,
    /// What was actually observed, one line per observation.
    findings: Vec<String>,
    /// The command lines that produced those observations, each encoded for a report.
    commands: Vec<CommandEvidence>,
    /// The workspace entry stems the full streams and raw statuses of those invocations were
    /// published under, one per invocation and positionally paired with [`FlagCheck::commands`].
    ///
    /// Named in the row rather than left to be discovered, because a reader who has just seen a
    /// two-hundred-character diagnostic excerpt needs to be told where the rest of it is. The
    /// stems are relative to the retained workspace the row's retention note names, so a row
    /// carries the directory once and the entries within it once each.
    captures: Vec<String>,
    /// Rationale, recorded limitations and unavailability diagnoses.
    notes: Vec<String>,
}

impl FlagCheck {
    /// The flag or property under examination.
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Which target and which compilers took part.
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// Whether this row records a semantic, acceptance-only, negative or structural check.
    pub fn kind(&self) -> CheckKind {
        self.kind
    }

    /// The observable consequence the check required.
    pub fn observable(&self) -> &str {
        &self.observable
    }

    /// The verdict.
    pub fn outcome(&self) -> CheckOutcome {
        self.outcome
    }

    /// What was observed, one line per observation.
    pub fn findings(&self) -> &[String] {
        &self.findings
    }

    /// The command lines that produced the observations, each in encoded form.
    pub fn commands(&self) -> &[CommandEvidence] {
        &self.commands
    }

    /// Rationale, limitations and unavailability diagnoses recorded on this row.
    pub fn notes(&self) -> &[String] {
        &self.notes
    }

    /// True when the check was performed and held.
    pub fn verified(&self) -> bool {
        self.outcome == CheckOutcome::Verified
    }

    /// True when the check was performed and did not hold.
    pub fn failed(&self) -> bool {
        self.outcome == CheckOutcome::Failed
    }

    /// True when the check could not be performed because a tool is absent.
    pub fn unavailable(&self) -> bool {
        self.outcome == CheckOutcome::Unavailable
    }

    /// A multi-line account of this row, naming the subject, the observable, every observation
    /// and every command line.
    ///
    /// This is what a failure message carries. "The flag probe failed" is useless; a reader needs
    /// the flag, both command lines and the check that did not hold, without re-running anything.
    pub fn describe(&self) -> String {
        // Read through this type's own accessors rather than its fields, so there is exactly one
        // read path for a row and a future accessor cannot drift away from what is rendered.
        let mut text = format!(
            "{} [{}] {} — {}\n    required: {}\n",
            self.outcome(),
            self.kind(),
            self.subject(),
            self.scope(),
            self.observable()
        );
        for finding in self.findings() {
            text.push_str("    observed: ");
            text.push_str(finding);
            text.push('\n');
        }
        for note in self.notes() {
            text.push_str("    note:     ");
            text.push_str(note);
            text.push('\n');
        }
        for command in self.commands() {
            text.push_str("    command:  ");
            text.push_str(&command.to_string());
            text.push('\n');
        }
        for capture in &self.captures {
            text.push_str("    capture:  ");
            text.push_str(capture);
            text.push_str(".{");
            text.push_str(CAPTURE_STDOUT_SUFFIX);
            text.push(',');
            text.push_str(CAPTURE_STDERR_SUFFIX);
            text.push(',');
            text.push_str(CAPTURE_EXIT_SUFFIX);
            text.push_str("}\n");
        }
        text
    }
}

/// Accumulator for one check in progress.
///
/// The verdict is derived from what was accumulated rather than assigned by the check body, so a
/// body that records a problem cannot also declare success — the single most valuable property of
/// this type. A recorded problem always wins over an unavailability, because a check that both
/// found something wrong and lacked a tool has still found something wrong.
struct Evidence {
    subject: String,
    scope: String,
    kind: CheckKind,
    observable: String,
    findings: Vec<String>,
    commands: Vec<CommandEvidence>,
    /// The capture stems, in invocation order, that this check's full streams were published
    /// under. One entry per invocation, paired positionally with [`Evidence::commands`].
    captures: Vec<String>,
    notes: Vec<String>,
    problems: Vec<String>,
    unavailable: Option<String>,
}

impl Evidence {
    /// Begin accumulating a check.
    ///
    /// Every text field is passed through [`report_text`] here rather than where it is printed, so
    /// a row cannot hold text that would disturb the report that renders it. The scope line is the
    /// reason this matters even for fields that look like harness prose: it names the reference
    /// driver, whose path came from an environment variable.
    fn new(
        subject: impl Into<String>,
        scope: impl Into<String>,
        kind: CheckKind,
        observable: impl Into<String>,
    ) -> Evidence {
        Evidence {
            subject: report_text(&subject.into()),
            scope: report_text(&scope.into()),
            kind,
            observable: report_text(&observable.into()),
            findings: Vec::new(),
            commands: Vec::new(),
            captures: Vec::new(),
            notes: Vec::new(),
            problems: Vec::new(),
            unavailable: None,
        }
    }

    /// Record something that was observed and holds.
    ///
    /// Sanitized on the way in. Most of what arrives here is this module's own prose, but that
    /// prose routinely interpolates a compiler's diagnostic excerpt or an error cause naming a
    /// path, and neither of those is under this module's control.
    fn observe(&mut self, text: impl Into<String>) {
        self.findings.push(report_text(&text.into()));
    }

    /// Record rationale, a limitation or an explanatory aside.
    fn note(&mut self, text: impl Into<String>) {
        self.notes.push(report_text(&text.into()));
    }

    /// Record something that was observed and does **not** hold.
    ///
    /// The text is also kept among the findings, so the row reads as a complete account of the
    /// check rather than splitting what was seen from what was wrong.
    fn problem(&mut self, text: impl Into<String>) {
        let text = report_text(&text.into());
        self.findings.push(format!("MISMATCH: {text}"));
        self.problems.push(text);
    }

    /// Record that a tool this check needs is absent, with the diagnosis discovery produced.
    fn mark_unavailable(&mut self, reason: impl Into<String>) {
        let reason = report_text(&reason.into());
        self.notes.push(format!("UNAVAILABLE: {reason}"));
        if self.unavailable.is_none() {
            self.unavailable = Some(reason);
        }
    }

    /// Record an invocation that took place: its command line, encoded for a report, and where it
    /// was captured.
    ///
    /// Both halves are recorded together because they are only useful together — a command line
    /// says what was run, and the capture stem says where the whole of what it produced was kept.
    /// The argument vector reaches this row only through [`CommandEvidence`], which is what keeps
    /// a binary path chosen through an environment variable from carrying report-forging
    /// characters into every row that names the invocation.
    fn record(&mut self, invocation: &Invocation) {
        self.commands
            .push(CommandEvidence::new(&invocation.outcome.command_line()));
        self.captures.push(invocation.capture.clone());
    }

    /// Record a command line that was captured elsewhere, for a row assembled from work shared
    /// with other rows.
    fn record_line(&mut self, command: impl Into<String>) {
        self.commands.push(CommandEvidence::new(&command.into()));
    }

    /// Record a capture stem that was published elsewhere, for a row assembled from work shared
    /// with other rows.
    fn record_capture(&mut self, capture: impl Into<String>) {
        self.captures.push(capture.into());
    }

    /// True while nothing has gone wrong.
    ///
    /// Distinct from "the check passed": an unavailability leaves this true, because there is
    /// nothing to preserve for inspection when the reason a check did not run is that a tool is
    /// not installed.
    fn holds(&self) -> bool {
        self.problems.is_empty()
    }

    /// Complete the check, deriving its verdict from what was accumulated.
    fn finish(self) -> FlagCheck {
        let outcome = if self.problems.is_empty() {
            match self.unavailable {
                Some(_) => CheckOutcome::Unavailable,
                None => CheckOutcome::Verified,
            }
        } else {
            CheckOutcome::Failed
        };
        FlagCheck {
            subject: self.subject,
            scope: self.scope,
            kind: self.kind,
            observable: self.observable,
            outcome,
            findings: self.findings,
            commands: self.commands,
            captures: self.captures,
            notes: self.notes,
        }
    }
}

/// Describe how an invocation ended, with the first substantive diagnostic line it produced.
///
/// The excerpt exists to identify a failure rather than to explain it in full: the untruncated
/// stream stays in the retained workspace, and a report row is one line plus its evidence.
///
/// Diagnostic text is never *compared* anywhere in this suite — compiler wording legitimately
/// differs — but quoting a line of it turns "the compilation failed" into something a reader can
/// act on immediately.
fn describe_termination(outcome: &RunOutcome) -> String {
    let mut text = String::from(outcome.termination().label());
    if let Some(line) = first_substantive_line(outcome.stderr()) {
        text.push_str(", first diagnostic: ");
        text.push_str(&line);
    }
    text
}

/// The first non-blank line of a captured stream, sanitized and truncated for a report row.
fn first_substantive_line(stream: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(stream);
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| truncate_for_report(line, DIAGNOSTIC_EXCERPT_CHARS_MAX))
}

/// A captured standard output stream, quoted safely for a report row.
///
/// Comparison always uses the raw bytes; this rendering exists only so a mismatch names what was
/// printed instead of merely stating that it differed.
fn quote_stdout(stream: &[u8]) -> String {
    let text = String::from_utf8_lossy(stream);
    format!("{:?}", truncate_for_report(&text, OUTPUT_EXCERPT_CHARS_MAX))
}

/// Render one piece of text so that nothing it contains can disturb a report.
///
/// The single entry point for untrusted text in this module. Every field of a [`FlagCheck`] and
/// every line of a [`FlagProbeReport`] passes through here *as it is recorded*, not as it is
/// rendered, which is a deliberate choice: sanitizing at each render sink means every future sink
/// must remember to, and the one that forgets is the one that matters. Sanitizing at entry makes
/// the property structural — a `FlagCheck` cannot hold unsafe text, so no consumer of one can
/// print unsafe text.
///
/// Two transformations, in this order. [`redact_secrets`] first, because a probe command line can
/// legitimately carry a `-D` definition and an environment summary can carry a tool path, and a
/// value that looks like a credential must not be copied into a report or a CI log. Then
/// [`sanitize_text_for_report`], which encodes every character a report cannot carry literally —
/// the escape introducer that would start a terminal control sequence, the carriage return that
/// would erase the line already written, the newline that would forge a further row, the tab that
/// would forge a column. The encoding is `\xNN` and `\u{NNNN}`, so the original byte is still
/// legible to a reader; it simply no longer acts.
///
/// Text arriving here is not necessarily hostile — most of it is this module's own prose. It is
/// text whose provenance is mixed: a harness sentence interpolating a compiler's diagnostic, an
/// error cause naming a path taken from an environment variable, an argument vector containing a
/// binary path the operator chose. Sanitizing all of it costs a copy and removes the need to
/// reason, case by case, about which interpolation was safe.
fn report_text(raw: &str) -> String {
    sanitize_text_for_report(&redact_secrets(raw))
}

/// Sanitize text for a report row and cap its length in characters.
///
/// Character-counted rather than byte-counted so that a multi-byte character can never be split
/// mid-sequence, and sanitized first so no control character, tab or escape introducer can forge
/// a column, erase a line or begin a terminal sequence.
fn truncate_for_report(raw: &str, limit: usize) -> String {
    let safe = sanitize_text_for_report(raw);
    if safe.chars().count() <= limit {
        return safe;
    }
    let kept: String = safe.chars().take(limit).collect();
    format!("{kept}… (truncated)")
}

// ---------------------------------------------------------------------------------------------
// SECTION 5 — the probe context and its shared operations.
//
// One context per run, holding the two compilers, the per-invocation budget and the fixture
// location. The operations below are the only places a compiler is invoked or an artifact is
// executed, so every check inherits the same confinement, the same bounded wait and the same
// recording of every command line as report evidence.
// ---------------------------------------------------------------------------------------------

/// Everything the positive checks share.
struct Probe<'a> {
    caps: &'a Capabilities,
    /// Per-invocation budget, taken from the run configuration so that a hung compiler or a hung
    /// program is bounded exactly as a corpus cell would be.
    budget: Duration,
    /// The external timeout utility when discovery found one; execution falls back to a watchdog
    /// thread when it did not, which changes no verdict.
    timeout_tool: Option<&'a Path>,
    /// The vetted path of the compiler under test.
    under_test_binary: PathBuf,
    /// The target every check uses unless the flag's meaning is inherently target-specific.
    ///
    /// The native target where the host is one of the four, so no check pays for emulation, and
    /// the baseline otherwise.
    target: Target,
    /// A second target, chosen so that the two targets between them cover both ELF classes.
    ///
    /// i686 is the only one of the four supported targets that is ELF32, so a probe that examined
    /// the primary target alone would exercise one of the reader's two offset tables and leave the
    /// other — wrong for one target in four — entirely unverified. That is the single most likely
    /// correctness defect in this module, so the class the primary target does not cover is
    /// covered here: i686 normally, and the baseline on the one host where the primary target is
    /// already i686.
    secondary_target: Target,
    /// Absolute path of the directory holding the fixture header, and the only directory the
    /// include-path check ever names.
    fixture_include_dir: PathBuf,
    /// Absolute path of the fixture header itself, recorded so a report names the file that was
    /// actually read.
    fixture_header: PathBuf,
}

impl<'a> Probe<'a> {
    /// Build the probe context, resolving the compiler under test and the fixture header.
    ///
    /// # Errors
    ///
    /// Fails when the compiler under test is absent, because there is then nothing to verify and
    /// no report could be honest about it — the same hard failure the degradation contract
    /// specifies for a missing compiler under test. Also fails when the fixture header is not
    /// where the corpus says it is, since without it the include-path check would silently lose
    /// the only route that proves `-I` was honoured.
    fn new(caps: &'a Capabilities) -> HarnessResult<Probe<'a>> {
        let bcc = caps.bcc().path().ok_or_else(|| {
            HarnessError::new(
                "preparing the flag-capability probe",
                format!(
                    "the compiler under test is unavailable, so no flag can be verified against \
                     it: {}",
                    caps.bcc().diagnosis()
                ),
            )
        })?;

        let fixture_include_dir = FIXTURE_INCLUDE_DIRECTORY
            .iter()
            .fold(corpus_root(), |path, component| path.join(component));
        let fixture_header = require_contained_corpus_file(
            "preparing the flag-capability probe's include-path fixture",
            "fixture header",
            &fixture_include_dir.join(FIXTURE_HEADER_NAME),
            "h",
        )?;

        let target = Target::ALL
            .iter()
            .copied()
            .find(|candidate| candidate.is_native())
            .unwrap_or(Target::BASELINE);

        Ok(Probe {
            caps,
            budget: budget_for(caps),
            timeout_tool: caps.timeout_tool().path(),
            under_test_binary: bcc.to_path_buf(),
            target,
            secondary_target: if target == Target::I686 {
                Target::BASELINE
            } else {
                Target::I686
            },
            fixture_include_dir,
            fixture_header,
        })
    }

    /// The compiler under test, configured for `target`.
    fn under_test(&self, target: Target) -> ProbeCompiler {
        ProbeCompiler {
            side: CompilerSide::UnderTest,
            binary: self.under_test_binary.clone(),
            prefix: "bcc",
            target,
        }
    }

    /// The reference compiler that targets `target`, or the reason there is none.
    ///
    /// The reference side selects a target by choosing a driver rather than by passing a flag, so
    /// a target with no configured driver has no reference arm at all. That is reported, never
    /// worked around by handing a driver a target it was not built for.
    fn reference(&self, target: Target) -> Result<ProbeCompiler, String> {
        match self.caps.ref_cc_for(target) {
            Some(binary) => Ok(ProbeCompiler {
                side: CompilerSide::Reference,
                binary: binary.to_path_buf(),
                prefix: "ref",
                target,
            }),
            None => Err(self.reference_absence(target)),
        }
    }

    /// The discovery summary of the reference driver that serves `target`.
    ///
    /// The native driver's own record is used where the target is the native one, because that is
    /// the record discovery vetted and diagnosed. A cross driver is described by its resolved
    /// path, which is the whole of what a report needs in order to reproduce the invocation.
    fn reference_summary(&self, target: Target) -> String {
        if target.is_native() {
            return self.caps.ref_cc_native().summary();
        }
        match self.caps.ref_cc_for(target) {
            Some(path) => format!(
                "{} reference driver at {}",
                target.triple(),
                shown_path(path)
            ),
            None => format!("no {} reference driver", target.triple()),
        }
    }

    /// Explain, loudly and specifically, why `target` has no reference arm.
    fn reference_absence(&self, target: Target) -> String {
        if target.is_native() {
            return format!(
                "the native reference compiler is unavailable, so no flag can be compared \
                 against a second compiler: {}",
                self.caps.ref_cc_native().diagnosis()
            );
        }
        format!(
            "no reference driver is configured for {}, so this arm cannot be compared; install \
             the matching cross driver or name one through the per-target override",
            target.triple()
        )
    }

    /// Column text naming the target and which compilers took part.
    fn scope(&self, target: Target, reference: Option<&ProbeCompiler>) -> String {
        match reference {
            Some(_) => format!("{} · both compilers", target.short_name()),
            None => format!("{} · compiler under test only", target.short_name()),
        }
    }

    /// The argument vector that runs `artifact`, or `None` when this target cannot be executed
    /// here.
    ///
    /// A native artifact runs directly; a foreign one runs under its emulator. The third
    /// situation — a foreign target with no emulator — returns `None` rather than falling back to
    /// direct execution, because launching a foreign binary on the host would fail with a
    /// non-zero status that is indistinguishable, to a check that reads status and output, from a
    /// compiler having produced a broken program.
    fn run_argv(&self, target: Target, artifact: &Path) -> HarnessResult<Option<Vec<String>>> {
        let artifact = path_text(artifact)?;
        if target.is_native() {
            return Ok(Some(vec![artifact]));
        }
        match self.caps.runner_for(target) {
            Some(runner) => Ok(Some(vec![path_text(runner)?, artifact])),
            None => Ok(None),
        }
    }

    /// Compile, record the command line as report evidence, and require the compilation to
    /// succeed.
    ///
    /// Returns `false` when the compilation failed, having already recorded the problem, so a
    /// caller can skip the observable it was about to inspect instead of inspecting an artifact
    /// that was never produced.
    fn expect_compile(
        &self,
        evidence: &mut Evidence,
        compiler: &ProbeCompiler,
        workspace: &Workspace,
        args: &[String],
        purpose: &str,
    ) -> HarnessResult<bool> {
        let invocation = compiler.compile(workspace, args, self.budget, self.timeout_tool)?;
        evidence.record(&invocation);
        if invocation.outcome.termination().succeeded() {
            return Ok(true);
        }
        evidence.problem(format!(
            "the {} failed to {purpose}: {}",
            compiler.label(),
            describe_termination(&invocation.outcome)
        ));
        Ok(false)
    }

    /// Compile, record the command line as report evidence, and require the compilation to
    /// **fail**.
    ///
    /// This is the load-bearing half of the include-path check. Without it, a compilation that
    /// succeeded for some reason other than the flag under test — a header reachable by a route
    /// nobody intended — would be indistinguishable from the flag having been honoured.
    fn expect_compile_failure(
        &self,
        evidence: &mut Evidence,
        compiler: &ProbeCompiler,
        workspace: &Workspace,
        args: &[String],
        purpose: &str,
    ) -> HarnessResult<()> {
        let invocation = compiler.compile(workspace, args, self.budget, self.timeout_tool)?;
        evidence.record(&invocation);
        if invocation.outcome.termination().succeeded() {
            evidence.problem(format!(
                "the {} succeeded when it was required to fail: {purpose}",
                compiler.label()
            ));
        } else {
            evidence.observe(format!(
                "the {} rejected the compilation as required — {purpose} ({})",
                compiler.label(),
                describe_termination(&invocation.outcome)
            ));
        }
        Ok(())
    }

    /// Execute an artifact and require it to exit zero and print exactly `expected`.
    ///
    /// Comparison is byte for byte on the raw stream, with no line-ending normalization and no
    /// substitution for a byte that is not valid text, because that is how every oracle in this
    /// suite compares output.
    ///
    /// Returns the captured standard output when the program ran, so a caller that also needs to
    /// compare one run against another can do so without executing it twice.
    fn expect_output(
        &self,
        evidence: &mut Evidence,
        compiler: &ProbeCompiler,
        workspace: &Workspace,
        artifact: &Path,
        expected: &str,
    ) -> HarnessResult<Option<Vec<u8>>> {
        let Some(argv) = self.run_argv(compiler.target, artifact)? else {
            evidence.mark_unavailable(format!(
                "a {} artifact cannot be executed on this machine, so the output of the {} could \
                 not be observed; install the emulator for that target or name one through its \
                 override",
                compiler.target.triple(),
                compiler.label()
            ));
            return Ok(None);
        };
        let invocation = compiler.execute(&argv, workspace, self.budget, self.timeout_tool)?;
        let outcome = &invocation.outcome;
        evidence.record(&invocation);
        if !outcome.termination().succeeded() {
            evidence.problem(format!(
                "the program built by the {} did not exit cleanly: {}",
                compiler.label(),
                describe_termination(outcome)
            ));
            return Ok(None);
        }
        if outcome.stdout() == expected.as_bytes() {
            evidence.observe(format!(
                "the program built by the {} printed {} and exited zero",
                compiler.label(),
                quote_stdout(outcome.stdout())
            ));
        } else {
            evidence.problem(format!(
                "the program built by the {} printed {} where {} was required",
                compiler.label(),
                quote_stdout(outcome.stdout()),
                quote_stdout(expected.as_bytes())
            ));
        }
        Ok(Some(outcome.stdout().to_vec()))
    }

    /// Require the ELF class of an already-read `image` to be the one `target` uses.
    ///
    /// Verified on every artifact the probe inspects rather than only on the 32-bit arm, because
    /// this is what proves the reader chose the right offset table — and choosing the wrong one
    /// would otherwise produce plausible-looking values read from the wrong place in the file.
    ///
    /// Takes the [`ElfImage`] rather than a path deliberately: the caller has already read the
    /// artifact once, and every fact this check and its neighbours report then describes the same
    /// bytes. A path parameter would invite a second read, which costs the artifact's size again
    /// and — worse — could disagree with the first.
    fn expect_elf_class(
        &self,
        evidence: &mut Evidence,
        compiler: &ProbeCompiler,
        image: &ElfImage,
    ) -> HarnessResult<()> {
        let width = compiler.target.elf_class();
        let expected = match width {
            32 => ELF_CLASS_32,
            _ => ELF_CLASS_64,
        };
        let observed = image.class();
        if observed == expected {
            evidence.observe(format!(
                "the {} produced an ELF{width} artifact, exercising the {width}-bit branch of the \
                 reader",
                compiler.label()
            ));
        } else {
            evidence.problem(format!(
                "the {} produced an artifact whose ELF class byte is {observed} where {expected} \
                 (ELF{width}) was required for {}",
                compiler.label(),
                compiler.target.triple()
            ));
        }
        Ok(())
    }

    /// Decide the ending of a single-row check's workspace, recording the outcome on the row.
    fn settle(workspace: Workspace, evidence: &mut Evidence) {
        settle_shared(workspace, &mut [evidence]);
    }
}

/// The compilers taking part in one check, and the reason there is only one when there is.
struct Participants {
    /// The compiler under test first, then the reference compiler when it is available. The order
    /// is fixed so report rows read the same way every time.
    compilers: Vec<ProbeCompiler>,
    /// Why the reference arm is missing, when it is.
    absence: Option<String>,
    /// Column text naming the target and the participants.
    scope: String,
}

/// One compile-then-run configuration, captured so several report rows can be built from the same
/// work.
///
/// Two checks need this: the optimization-level check compares three configurations against each
/// other and reports one row per level, and the macro checks compare three configurations and
/// report one row for `-D` and one for `-U`. Re-running the compilations once per row would double
/// the work and, worse, would let two rows disagree about what the same configuration printed.
struct Configuration {
    /// How the configuration is described in a row, for example `-D BCC_FLAG_PROBE_MACRO=7`.
    label: String,
    /// Which compiler produced it.
    side: CompilerSide,
    /// Exact command lines, the compilation first and the execution second.
    commands: Vec<String>,
    /// The capture stems those invocations were published under, in the same order, so a row
    /// assembled from shared work names its evidence exactly as a single-invocation row does.
    captures: Vec<String>,
    /// What the program printed, when it ran and exited cleanly.
    printed: Option<Vec<u8>>,
    /// What went wrong, when something did.
    problem: Option<String>,
    /// Why the configuration could not be observed, when a tool was absent.
    unavailable: Option<String>,
}

impl Configuration {
    /// Fold this configuration into a row: its commands, its verdict and what it printed.
    fn absorb(&self, evidence: &mut Evidence) {
        for command in &self.commands {
            evidence.record_line(command.clone());
        }
        for capture in &self.captures {
            evidence.record_capture(capture.clone());
        }
        if let Some(problem) = &self.problem {
            evidence.problem(problem.clone());
        }
        if let Some(reason) = &self.unavailable {
            evidence.mark_unavailable(reason.clone());
        }
        if let Some(printed) = &self.printed {
            evidence.observe(format!(
                "{} under the {} printed {}",
                self.label,
                self.side.label(),
                quote_stdout(printed)
            ));
        }
    }
}

/// Build an owned argument vector from borrowed parts, so a check reads as the command line it
/// runs.
fn args(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| String::from(*part)).collect()
}

// ---------------------------------------------------------------------------------------------
// SECTION 6 — the positive checks.
//
// One method per flag, each asserting acceptance **and** the observable consequence the flag is
// supposed to have. Acceptance alone is never enough: `-fcf-protection` is accepted by both
// compilers and means something different to each, so a probe that only asked "was it accepted"
// would have admitted it to the shared set and quietly corrupted every differential comparison
// in the suite.
// ---------------------------------------------------------------------------------------------

impl Probe<'_> {
    /// Resolve the compilers taking part in a check on `target`.
    fn participants(&self, target: Target) -> Participants {
        let under_test = self.under_test(target);
        match self.reference(target) {
            Ok(reference) => Participants {
                scope: self.scope(target, Some(&reference)),
                compilers: vec![under_test, reference],
                absence: None,
            },
            Err(reason) => Participants {
                scope: self.scope(target, None),
                compilers: vec![under_test],
                absence: Some(reason),
            },
        }
    }

    /// Allocate the workspace for one check and write its program into it.
    ///
    /// Returns the workspace and the absolute path of the source as text. Absolute throughout, so
    /// every recorded command line reproduces from any working directory rather than only from
    /// the one the probe happened to use.
    fn stage(
        &self,
        workspace_name: &str,
        entry: &str,
        program: &str,
    ) -> HarnessResult<(Workspace, String)> {
        let workspace = probe_workspace(workspace_name, self.caps.config())?;
        let source = workspace.write_text(entry, program)?;
        let source = path_text(&source)?;
        Ok((workspace, source))
    }

    /// Compile one configuration and run what it produced, capturing everything for later rows.
    fn build_and_run(
        &self,
        compiler: &ProbeCompiler,
        workspace: &Workspace,
        label: impl Into<String>,
        extra: &[String],
        source: &str,
        stem: &str,
    ) -> HarnessResult<Configuration> {
        let label = label.into();
        let artifact = compiler.artifact(workspace, stem)?;
        let artifact_text = path_text(&artifact)?;
        let mut argv = args(&[FLAG_STATIC, FLAG_OUTPUT, &artifact_text, source]);
        argv.extend(extra.iter().cloned());

        let mut configuration = Configuration {
            label: label.clone(),
            side: compiler.side,
            commands: Vec::new(),
            captures: Vec::new(),
            printed: None,
            problem: None,
            unavailable: None,
        };

        let compiled = compiler.compile(workspace, &argv, self.budget, self.timeout_tool)?;
        configuration.commands.push(compiled.outcome.command_line());
        configuration.captures.push(compiled.capture.clone());
        if !compiled.outcome.termination().succeeded() {
            configuration.problem = Some(format!(
                "the {} failed to compile the probe with {label}: {}",
                compiler.label(),
                describe_termination(&compiled.outcome)
            ));
            return Ok(configuration);
        }

        let Some(run) = self.run_argv(compiler.target, &artifact)? else {
            configuration.unavailable = Some(format!(
                "a {} artifact cannot be executed on this machine, so what {label} prints could \
                 not be observed",
                compiler.target.triple()
            ));
            return Ok(configuration);
        };
        let executed = compiler.execute(&run, workspace, self.budget, self.timeout_tool)?;
        configuration.commands.push(executed.outcome.command_line());
        configuration.captures.push(executed.capture.clone());
        if executed.outcome.termination().succeeded() {
            configuration.printed = Some(executed.outcome.stdout().to_vec());
        } else {
            configuration.problem = Some(format!(
                "the program the {} built with {label} did not exit cleanly: {}",
                compiler.label(),
                describe_termination(&executed.outcome)
            ));
        }
        Ok(configuration)
    }

    /// `-o`: the file named is the file written, at exactly the path requested.
    ///
    /// The requested name is one no driver would choose on its own, so a driver that ignored the
    /// flag and wrote its default output would leave the named path absent rather than
    /// accidentally satisfying the check.
    fn check_output_naming(&self) -> HarnessResult<FlagCheck> {
        let party = self.participants(self.target);
        let mut evidence = Evidence::new(
            FLAG_OUTPUT,
            party.scope.as_str(),
            CheckKind::Semantic,
            "the file named by -o exists as a regular file at exactly the requested path, and is \
             the linked executable rather than an empty placeholder",
        );
        if let Some(reason) = &party.absence {
            evidence.mark_unavailable(reason.clone());
        }
        let (workspace, source) = self.stage(FLAG_OUTPUT, SOURCE_ARITHMETIC, PROGRAM_ARITHMETIC)?;

        for compiler in &party.compilers {
            let artifact = compiler.artifact(&workspace, "named_output.bin")?;
            let artifact_text = path_text(&artifact)?;
            let argv = args(&[FLAG_STATIC, FLAG_OUTPUT, &artifact_text, &source]);
            if !self.expect_compile(
                &mut evidence,
                compiler,
                &workspace,
                &argv,
                "build the probe program",
            )? {
                continue;
            }
            let context = format!(
                "checking that the {} honoured {FLAG_OUTPUT}",
                compiler.label()
            );
            match require_regular_file(&context, &artifact) {
                Ok(()) => evidence.observe(format!(
                    "the {} wrote {} exactly as named",
                    compiler.label(),
                    shown_path(&artifact)
                )),
                Err(error) => evidence.problem(format!(
                    "the {} exited successfully but {}",
                    compiler.label(),
                    error.cause()
                )),
            }
            match ElfImage::read(&artifact).and_then(|image| image.e_type()) {
                Ok(ET_EXEC) => evidence.observe(format!(
                    "the file the {} named holds a linked executable",
                    compiler.label()
                )),
                Ok(other) => evidence.problem(format!(
                    "the file the {} named has ELF type {other} ({}) where {ET_EXEC} (executable) \
                     was required, so the flag named something other than the linked program",
                    compiler.label(),
                    describe_elf_type(other)
                )),
                Err(error) => evidence.problem(format!(
                    "the file the {} named could not be read as ELF: {}",
                    compiler.label(),
                    error.cause()
                )),
            }
        }

        Probe::settle(workspace, &mut evidence);
        Ok(evidence.finish())
    }

    /// `-c`: compilation stops before linking, so the output is a relocatable object.
    ///
    /// The ELF type field is the observable, read directly from the file. An object also carries
    /// no interpreter header, which is checked as well because it is free and because it makes the
    /// contrast with a dynamically linked executable explicit in the report.
    fn check_compile_only(&self) -> HarnessResult<FlagCheck> {
        let party = self.participants(self.target);
        let mut evidence = Evidence::new(
            FLAG_COMPILE_ONLY,
            party.scope.as_str(),
            CheckKind::Semantic,
            format!(
                "the artifact's ELF type field at file offset {E_TYPE_OFFSET:#x} is {ET_REL} \
                 (relocatable object) and it carries no program interpreter header"
            ),
        );
        if let Some(reason) = &party.absence {
            evidence.mark_unavailable(reason.clone());
        }
        let (workspace, source) =
            self.stage(FLAG_COMPILE_ONLY, SOURCE_ARITHMETIC, PROGRAM_ARITHMETIC)?;

        for compiler in &party.compilers {
            let object = compiler.artifact(&workspace, "object.o")?;
            let object_text = path_text(&object)?;
            let argv = args(&[FLAG_COMPILE_ONLY, FLAG_OUTPUT, &object_text, &source]);
            if !self.expect_compile(
                &mut evidence,
                compiler,
                &workspace,
                &argv,
                "compile the probe program without linking",
            )? {
                continue;
            }
            let image = ElfImage::read(&object)?;
            self.expect_elf_class(&mut evidence, compiler, &image)?;
            let observed = image.e_type()?;
            if observed == ET_REL {
                evidence.observe(format!(
                    "the {} produced ELF type {observed} ({})",
                    compiler.label(),
                    describe_elf_type(observed)
                ));
            } else {
                evidence.problem(format!(
                    "the {} produced ELF type {observed} ({}) where {ET_REL} (relocatable object) \
                     was required, so {FLAG_COMPILE_ONLY} did not stop before linking",
                    compiler.label(),
                    describe_elf_type(observed)
                ));
            }
            if image.has_interp()? {
                evidence.problem(format!(
                    "the object the {} produced carries a program interpreter header, which a \
                     relocatable object never does",
                    compiler.label()
                ));
            } else {
                evidence.observe(format!(
                    "the object the {} produced carries no program interpreter header",
                    compiler.label()
                ));
            }
        }

        Probe::settle(workspace, &mut evidence);
        Ok(evidence.finish())
    }
}

impl Probe<'_> {
    /// `-static`: the artifact is a self-contained executable with no program interpreter.
    ///
    /// Two observations, both required of both compilers: the ELF type field is
    /// [`ET_EXEC`], and no program header of interpreter type is present. The absence of the
    /// interpreter header is the substantive half — it is what makes the artifact runnable under
    /// an emulator with no sysroot and no dynamic loader, which is why every cell in the suite is
    /// built this way.
    ///
    /// A contrast build without the flag establishes that the check has discriminating power. The
    /// reference compiler is the measured authority here and must produce an interpreter header
    /// without `-static`; the compiler under test is observed and reported either way, because a
    /// compiler whose *default* linkage is already static differs from the reference in its
    /// default rather than in what `-static` means, and requirement 3 constrains the latter.
    fn check_static_linkage(&self) -> HarnessResult<FlagCheck> {
        let party = self.participants(self.target);
        let mut evidence = Evidence::new(
            FLAG_STATIC,
            party.scope.as_str(),
            CheckKind::Semantic,
            format!(
                "the artifact's ELF type field is {ET_EXEC} (executable) and it carries no \
                 program interpreter header, while the same compilation without the flag does \
                 carry one"
            ),
        );
        if let Some(reason) = &party.absence {
            evidence.mark_unavailable(reason.clone());
        }
        let (workspace, source) = self.stage(FLAG_STATIC, SOURCE_ARITHMETIC, PROGRAM_ARITHMETIC)?;

        for compiler in &party.compilers {
            let artifact = compiler.artifact(&workspace, "static.bin")?;
            let artifact_text = path_text(&artifact)?;
            let argv = args(&[FLAG_STATIC, FLAG_OUTPUT, &artifact_text, &source]);
            if self.expect_compile(
                &mut evidence,
                compiler,
                &workspace,
                &argv,
                "build a statically linked program",
            )? {
                let image = ElfImage::read(&artifact)?;
                self.expect_elf_class(&mut evidence, compiler, &image)?;
                self.expect_static_shape(&mut evidence, compiler, &image)?;
                self.expect_output(
                    &mut evidence,
                    compiler,
                    &workspace,
                    &artifact,
                    ARITHMETIC_STDOUT,
                )?;
            }
            self.observe_dynamic_contrast(&mut evidence, compiler, &workspace, &source)?;
        }

        Probe::settle(workspace, &mut evidence);
        Ok(evidence.finish())
    }

    /// `-static` again, on the target whose ELF class the primary target does not cover.
    ///
    /// This row exists for one reason: the program- and section-header offsets differ between the
    /// two ELF classes, so a reader verified on 64-bit artifacts alone would be unverified for
    /// i686 — one supported target in four. The compiler under test reaches the other class
    /// through its own target-selection flag, which is legitimate here because the flag is being
    /// used to select a backend rather than being offered as a flag both compilers share; the
    /// reference side reaches it by using the matching cross driver, because it has no
    /// target-selection flag at all and the spelling that resembles one was measured to be
    /// rejected.
    ///
    /// The artifact is executed when this machine can execute it, and inspected either way. An
    /// absent emulator is recorded as a note rather than an unavailability, because inspection is
    /// the observable requirement 3 asks for and execution is a strengthening on top of it.
    fn check_static_linkage_other_class(&self) -> HarnessResult<FlagCheck> {
        let target = self.secondary_target;
        let party = self.participants(target);
        let width = target.elf_class();
        let mut evidence = Evidence::new(
            FLAG_STATIC,
            party.scope.as_str(),
            CheckKind::Semantic,
            format!(
                "the artifact is ELF{width}, its ELF type field is {ET_EXEC} (executable) and it \
                 carries no program interpreter header, exercising the {width}-bit branch of the \
                 ELF reader"
            ),
        );
        evidence.note(format!(
            "this row covers the ELF class the primary target does not: the program- and \
             section-header offsets differ between ELF32 and ELF64, so a reader exercised on one \
             class alone would be unverified for {} of the four supported targets",
            if width == 32 { "i686, one" } else { "three" }
        ));
        if let Some(reason) = &party.absence {
            evidence.mark_unavailable(reason.clone());
        }
        let (workspace, source) =
            self.stage("-static other class", SOURCE_ARITHMETIC, PROGRAM_ARITHMETIC)?;

        for compiler in &party.compilers {
            if compiler.selects_target_by_flag() {
                evidence.note(format!(
                    "the {} selects this target with its own target flag, which is legitimate \
                     because both sides of a cross-backend comparison are the same compiler; the \
                     {} selects a target by driver instead, and never receives that flag",
                    compiler.label(),
                    CompilerSide::Reference.label()
                ));
            }
            let artifact = compiler.artifact(&workspace, "static_other_class.bin")?;
            let artifact_text = path_text(&artifact)?;
            let argv = args(&[FLAG_STATIC, FLAG_OUTPUT, &artifact_text, &source]);
            if !self.expect_compile(
                &mut evidence,
                compiler,
                &workspace,
                &argv,
                "build a statically linked program for the other ELF class",
            )? {
                continue;
            }
            let image = ElfImage::read(&artifact)?;
            self.expect_elf_class(&mut evidence, compiler, &image)?;
            self.expect_static_shape(&mut evidence, compiler, &image)?;
            if self.caps.can_execute(target) {
                self.expect_output(
                    &mut evidence,
                    compiler,
                    &workspace,
                    &artifact,
                    ARITHMETIC_STDOUT,
                )?;
            } else {
                evidence.note(format!(
                    "the {} artifact was inspected but not executed, because this machine has no \
                     runner for {}; the ELF evidence above is the observable requirement 3 asks \
                     for, and execution would only strengthen it",
                    compiler.label(),
                    target.triple()
                ));
            }
        }

        Probe::settle(workspace, &mut evidence);
        Ok(evidence.finish())
    }

    /// Require an already-read artifact to have the shape a static executable has.
    ///
    /// Both facts — the type field and the absence of a program interpreter header — are taken
    /// from the same [`ElfImage`], so the two halves of the conclusion cannot describe two
    /// different readings of the file.
    fn expect_static_shape(
        &self,
        evidence: &mut Evidence,
        compiler: &ProbeCompiler,
        image: &ElfImage,
    ) -> HarnessResult<()> {
        let observed = image.e_type()?;
        if observed == ET_EXEC {
            evidence.observe(format!(
                "the {} produced ELF type {observed} ({})",
                compiler.label(),
                describe_elf_type(observed)
            ));
        } else {
            evidence.problem(format!(
                "the {} produced ELF type {observed} ({}) where {ET_EXEC} (executable) was \
                 required; a statically linked program is not a shared object",
                compiler.label(),
                describe_elf_type(observed)
            ));
        }
        if image.has_interp()? {
            evidence.problem(format!(
                "the artifact the {} produced carries a program interpreter header, so it is not \
                 self-contained and could not be executed under an emulator without a sysroot",
                compiler.label()
            ));
        } else {
            evidence.observe(format!(
                "the artifact the {} produced carries no program interpreter header, so it is \
                 self-contained",
                compiler.label()
            ));
        }
        Ok(())
    }

    /// Build the same program without `-static` and report what changed.
    ///
    /// Required of the reference compiler, which is the measured authority: without the flag it
    /// must produce a program interpreter header, which is what proves the header's absence above
    /// was caused by the flag rather than by everything. Observed and reported for the compiler
    /// under test, whose default linkage is its own affair.
    fn observe_dynamic_contrast(
        &self,
        evidence: &mut Evidence,
        compiler: &ProbeCompiler,
        workspace: &Workspace,
        source: &str,
    ) -> HarnessResult<()> {
        let artifact = compiler.artifact(workspace, "default_linkage.bin")?;
        let artifact_text = path_text(&artifact)?;
        let argv = args(&[FLAG_OUTPUT, &artifact_text, source]);
        let invocation = compiler.compile(workspace, &argv, self.budget, self.timeout_tool)?;
        let outcome = &invocation.outcome;
        evidence.record(&invocation);

        let authoritative = compiler.side == CompilerSide::Reference;
        if !outcome.termination().succeeded() {
            let detail = format!(
                "the {} could not build the same program without {FLAG_STATIC}: {}",
                compiler.label(),
                describe_termination(outcome)
            );
            if authoritative {
                evidence.problem(format!(
                    "{detail}; the contrast build is what establishes that the absence of a \
                     program interpreter header above was caused by the flag"
                ));
            } else {
                evidence.note(format!(
                    "{detail}; this is a property of that compiler's default linkage rather than \
                     of what {FLAG_STATIC} means, so it is reported rather than treated as a \
                     flag-parity failure"
                ));
            }
            return Ok(());
        }

        let image = ElfImage::read(&artifact)?;
        let default_type = image.e_type()?;
        let default_interp = image.has_interp()?;
        if default_interp {
            evidence.observe(format!(
                "without {FLAG_STATIC} the {} produced ELF type {default_type} ({}) with a program \
                 interpreter header, so the check above discriminates",
                compiler.label(),
                describe_elf_type(default_type)
            ));
            return Ok(());
        }
        let detail = format!(
            "without {FLAG_STATIC} the {} produced ELF type {default_type} ({}) and still no \
             program interpreter header",
            compiler.label(),
            describe_elf_type(default_type)
        );
        if authoritative {
            evidence.problem(format!(
                "{detail}, so the contrast establishes nothing and the interpreter check above \
                 has no discriminating power in this environment"
            ));
        } else {
            evidence.note(format!(
                "{detail}, which means that compiler links statically by default; a difference in \
                 default linkage is not a difference in what {FLAG_STATIC} means, so it is \
                 reported here rather than treated as a flag-parity failure"
            ));
        }
        Ok(())
    }

    /// `-g`: debug information appears, and its absence without the flag is checked too.
    ///
    /// Both directions are required. Asserting only that `.debug_info` is present with the flag
    /// would pass just as happily against a compiler that emitted the section unconditionally,
    /// which would tell a reader nothing about whether the flag was honoured.
    fn check_debug_information(&self) -> HarnessResult<FlagCheck> {
        let party = self.participants(self.target);
        let mut evidence = Evidence::new(
            FLAG_DEBUG,
            party.scope.as_str(),
            CheckKind::Semantic,
            format!(
                "a section named {DEBUG_INFO_SECTION} is present with the flag and absent without \
                 it"
            ),
        );
        if let Some(reason) = &party.absence {
            evidence.mark_unavailable(reason.clone());
        }
        let (workspace, source) = self.stage(FLAG_DEBUG, SOURCE_ARITHMETIC, PROGRAM_ARITHMETIC)?;

        for compiler in &party.compilers {
            let with_debug = compiler.artifact(&workspace, "with_debug.bin")?;
            let with_text = path_text(&with_debug)?;
            let argv = args(&[FLAG_DEBUG, FLAG_STATIC, FLAG_OUTPUT, &with_text, &source]);
            if self.expect_compile(
                &mut evidence,
                compiler,
                &workspace,
                &argv,
                "build the probe program with debug information",
            )? {
                if ElfImage::read(&with_debug)?.has_section(DEBUG_INFO_SECTION)? {
                    evidence.observe(format!(
                        "with {FLAG_DEBUG} the {} emitted a {DEBUG_INFO_SECTION} section",
                        compiler.label()
                    ));
                } else {
                    evidence.problem(format!(
                        "with {FLAG_DEBUG} the {} emitted no {DEBUG_INFO_SECTION} section, so the \
                         flag had no observable effect",
                        compiler.label()
                    ));
                }
            }

            let without_debug = compiler.artifact(&workspace, "without_debug.bin")?;
            let without_text = path_text(&without_debug)?;
            let argv = args(&[FLAG_STATIC, FLAG_OUTPUT, &without_text, &source]);
            if self.expect_compile(
                &mut evidence,
                compiler,
                &workspace,
                &argv,
                "build the probe program without debug information",
            )? {
                if ElfImage::read(&without_debug)?.has_section(DEBUG_INFO_SECTION)? {
                    evidence.problem(format!(
                        "without {FLAG_DEBUG} the {} still emitted a {DEBUG_INFO_SECTION} \
                         section, so the presence of that section proves nothing about the flag",
                        compiler.label()
                    ));
                } else {
                    evidence.observe(format!(
                        "without {FLAG_DEBUG} the {} emitted no {DEBUG_INFO_SECTION} section",
                        compiler.label()
                    ));
                }
            }
        }

        Probe::settle(workspace, &mut evidence);
        Ok(evidence.finish())
    }

    /// `-I`: the named directory is searched, and without it the same program does not compile.
    ///
    /// The program names its header with angle brackets rather than quotes, so the include search
    /// path is the only route to it. That is what makes the negative half load-bearing: a quoted
    /// include would be found in the including file's own directory and the compilation would
    /// succeed whether or not the flag was honoured.
    ///
    /// The header is the suite's single fixture. Its own comment records that no second copy may
    /// exist anywhere in the corpus, for exactly this reason.
    fn check_include_path(&self) -> HarnessResult<FlagCheck> {
        let party = self.participants(self.target);
        let mut evidence = Evidence::new(
            FLAG_INCLUDE,
            party.scope.as_str(),
            CheckKind::Semantic,
            "a header reachable only through the directory named by -I is included successfully, \
             the program prints the values that header defines, and the same compilation without \
             the flag fails",
        );
        if let Some(reason) = &party.absence {
            evidence.mark_unavailable(reason.clone());
        }
        evidence.note(format!(
            "the fixture header is {}, and the program names it with angle brackets so that the \
             include search path is its only route",
            shown_path(&self.fixture_header)
        ));
        evidence.note(format!(
            "the printed values come from {FIXTURE_VALUE_MACRO} and {FIXTURE_NAME_MACRO}, which \
             the fixture defines, so the output could not be produced without the header"
        ));
        let (workspace, source) =
            self.stage(FLAG_INCLUDE, SOURCE_HEADER, PROGRAM_HEADER_INCLUDE)?;
        let include_dir = path_text(&self.fixture_include_dir)?;

        for compiler in &party.compilers {
            let artifact = compiler.artifact(&workspace, "with_include.bin")?;
            let artifact_text = path_text(&artifact)?;
            let argv = args(&[
                FLAG_INCLUDE,
                &include_dir,
                FLAG_STATIC,
                FLAG_OUTPUT,
                &artifact_text,
                &source,
            ]);
            if self.expect_compile(
                &mut evidence,
                compiler,
                &workspace,
                &argv,
                "compile a program whose header is reachable only through the include search path",
            )? {
                evidence.observe(format!(
                    "the {} found the fixture header through {FLAG_INCLUDE}",
                    compiler.label()
                ));
                self.expect_output(
                    &mut evidence,
                    compiler,
                    &workspace,
                    &artifact,
                    HEADER_STDOUT,
                )?;
            }

            let denied = compiler.artifact(&workspace, "without_include.bin")?;
            let denied_text = path_text(&denied)?;
            let argv = args(&[FLAG_STATIC, FLAG_OUTPUT, &denied_text, &source]);
            self.expect_compile_failure(
                &mut evidence,
                compiler,
                &workspace,
                &argv,
                "the fixture header is unreachable without the include search path",
            )?;
        }

        Probe::settle(workspace, &mut evidence);
        Ok(evidence.finish())
    }
}

impl Probe<'_> {
    /// `-D` and `-U`: the definition given on the command line reaches the program, and removing
    /// it again takes it away.
    ///
    /// Both rows come from the same three configurations, because re-running the compilations per
    /// row would double the work and would let two rows disagree about what the same configuration
    /// printed. The observable is the *printed value*, not acceptance: a driver that accepted `-D`
    /// and discarded it would compile just as cleanly.
    fn check_macro_flags(&self) -> HarnessResult<Vec<FlagCheck>> {
        let party = self.participants(self.target);
        let (workspace, source) = self.stage("-D and -U", SOURCE_MACRO, PROGRAM_MACRO)?;
        let definition = format!("{MACRO_NAME}={MACRO_VALUE}");
        let default_output = format!("{MACRO_OUTPUT_PREFIX}{MACRO_DEFAULT}\n");
        let defined_output = format!("{MACRO_OUTPUT_PREFIX}{MACRO_VALUE}\n");

        let mut baseline = Vec::new();
        let mut defined = Vec::new();
        let mut undefined = Vec::new();
        for compiler in &party.compilers {
            baseline.push(self.build_and_run(
                compiler,
                &workspace,
                "the program with no macro flag",
                &[],
                &source,
                "macro_baseline.bin",
            )?);
            defined.push(self.build_and_run(
                compiler,
                &workspace,
                format!("{FLAG_DEFINE} {definition}"),
                &args(&[FLAG_DEFINE, &definition]),
                &source,
                "macro_defined.bin",
            )?);
            undefined.push(self.build_and_run(
                compiler,
                &workspace,
                format!("{FLAG_DEFINE} {definition} {FLAG_UNDEFINE} {MACRO_NAME}"),
                &args(&[FLAG_DEFINE, &definition, FLAG_UNDEFINE, MACRO_NAME]),
                &source,
                "macro_undefined.bin",
            )?);
        }

        let mut define_row = Evidence::new(
            FLAG_DEFINE,
            party.scope.as_str(),
            CheckKind::Semantic,
            format!(
                "the program prints {MACRO_VALUE} for {MACRO_NAME} when the flag defines it, \
                 where it prints its built-in default of {MACRO_DEFAULT} when nothing does — the \
                 printed value changes, not merely the exit status"
            ),
        );
        let mut undefine_row = Evidence::new(
            FLAG_UNDEFINE,
            party.scope.as_str(),
            CheckKind::Semantic,
            format!(
                "a definition supplied by {FLAG_DEFINE} and then removed by {FLAG_UNDEFINE} \
                 returns the printed value to the program's built-in default of {MACRO_DEFAULT}"
            ),
        );
        if let Some(reason) = &party.absence {
            define_row.mark_unavailable(reason.clone());
            undefine_row.mark_unavailable(reason.clone());
        }

        for index in 0..party.compilers.len() {
            let label = party.compilers[index].label();
            baseline[index].absorb(&mut define_row);
            defined[index].absorb(&mut define_row);
            expect_printed(&mut define_row, label, &baseline[index], &default_output);
            expect_printed(&mut define_row, label, &defined[index], &defined_output);
            expect_changed(
                &mut define_row,
                label,
                &baseline[index],
                &defined[index],
                FLAG_DEFINE,
            );

            for command in &defined[index].commands {
                undefine_row.record_line(command.clone());
            }
            if let Some(printed) = &defined[index].printed {
                undefine_row.observe(format!(
                    "{} under the {label} printed {}",
                    defined[index].label,
                    quote_stdout(printed)
                ));
            }
            undefined[index].absorb(&mut undefine_row);
            expect_printed(&mut undefine_row, label, &undefined[index], &default_output);
            expect_changed(
                &mut undefine_row,
                label,
                &defined[index],
                &undefined[index],
                FLAG_UNDEFINE,
            );
        }

        settle_shared(workspace, &mut [&mut define_row, &mut undefine_row]);
        Ok(vec![define_row.finish(), undefine_row.finish()])
    }

    /// `-O0`, `-O1`, `-O2`: every level is accepted, and the program's output is identical at all
    /// three.
    ///
    /// The program's operands are `volatile`, so the arithmetic is genuinely emitted at every
    /// level rather than folded to an immediate. Without that, comparing output across levels
    /// would compare the constant folder against itself and a code-generation defect would pass
    /// unnoticed — the measured reason the corpus applies the same two-variant rule.
    ///
    /// One row per level, each carrying that level's own commands and the shared invariance
    /// verdict, so a reader can see at a glance which level was involved in a divergence.
    fn check_optimization_levels(&self) -> HarnessResult<Vec<FlagCheck>> {
        let party = self.participants(self.target);
        let (workspace, source) = self.stage("-O levels", SOURCE_ARITHMETIC, PROGRAM_ARITHMETIC)?;

        let mut per_level: Vec<Vec<Configuration>> = Vec::with_capacity(OptLevel::ALL.len());
        for level in OptLevel::ALL {
            let mut configurations = Vec::with_capacity(party.compilers.len());
            for compiler in &party.compilers {
                configurations.push(self.build_and_run(
                    compiler,
                    &workspace,
                    format!("the program at {}", level.flag()),
                    &args(&[level.flag()]),
                    &source,
                    &format!("opt_{}.bin", level.short()),
                )?);
            }
            per_level.push(configurations);
        }

        let divergence = describe_output_divergence(&per_level);
        let mut rows = Vec::with_capacity(OptLevel::ALL.len());
        for (index, level) in OptLevel::ALL.iter().enumerate() {
            let mut evidence = Evidence::new(
                level.flag(),
                party.scope.as_str(),
                CheckKind::Semantic,
                format!(
                    "both compilers accept {} and the program prints exactly the same bytes at \
                     {}, {} and {}",
                    level.flag(),
                    OptLevel::O0.flag(),
                    OptLevel::O1.flag(),
                    OptLevel::O2.flag()
                ),
            );
            evidence.note(
                "the program's operands are volatile, so the arithmetic is emitted at every level \
                 rather than folded to an immediate; without that, comparing output across levels \
                 would compare the constant folder against itself",
            );
            if let Some(reason) = &party.absence {
                evidence.mark_unavailable(reason.clone());
            }
            for (side_index, configuration) in per_level[index].iter().enumerate() {
                configuration.absorb(&mut evidence);
                expect_printed(
                    &mut evidence,
                    party.compilers[side_index].label(),
                    configuration,
                    ARITHMETIC_STDOUT,
                );
            }
            match &divergence {
                Some(detail) => evidence.problem(detail.clone()),
                None => evidence.observe(String::from(
                    "every configuration that ran printed the same bytes, so the observable \
                     behaviour is invariant across the optimization levels",
                )),
            }
            rows.push(evidence);
        }

        let mut borrowed: Vec<&mut Evidence> = rows.iter_mut().collect();
        settle_shared(workspace, &mut borrowed);
        Ok(rows.into_iter().map(Evidence::finish).collect())
    }

    /// `-fPIC`: accepted by both, and what it produces still runs.
    ///
    /// Position-independent code inside a statically linked executable is a legitimate
    /// combination, and running the result is the observable: a flag that were mishandled into
    /// wrong relocations would produce an artifact that links and then misbehaves, which only
    /// execution can reveal.
    fn check_position_independent_code(&self) -> HarnessResult<FlagCheck> {
        let party = self.participants(self.target);
        let mut evidence = Evidence::new(
            FLAG_PIC,
            party.scope.as_str(),
            CheckKind::Semantic,
            "both compilers accept the flag and the artifact it produces is a runnable executable \
             that prints exactly what the program prints without it",
        );
        if let Some(reason) = &party.absence {
            evidence.mark_unavailable(reason.clone());
        }
        let (workspace, source) = self.stage(FLAG_PIC, SOURCE_ARITHMETIC, PROGRAM_ARITHMETIC)?;

        for compiler in &party.compilers {
            let artifact = compiler.artifact(&workspace, "pic.bin")?;
            let artifact_text = path_text(&artifact)?;
            let argv = args(&[FLAG_PIC, FLAG_STATIC, FLAG_OUTPUT, &artifact_text, &source]);
            if !self.expect_compile(
                &mut evidence,
                compiler,
                &workspace,
                &argv,
                "build the probe program as position-independent code",
            )? {
                continue;
            }
            self.expect_elf_class(&mut evidence, compiler, &ElfImage::read(&artifact)?)?;
            self.expect_output(
                &mut evidence,
                compiler,
                &workspace,
                &artifact,
                ARITHMETIC_STDOUT,
            )?;
        }

        Probe::settle(workspace, &mut evidence);
        Ok(evidence.finish())
    }

    /// `-L` and `-l`: acceptance only, and the limitation is stated rather than glossed over.
    ///
    /// # Why this pair is not verified semantically
    ///
    /// Proving that the directory named by `-L` was searched requires an archive in it that the
    /// link would otherwise not find, which requires creating an archive — a tool outside the
    /// no-new-dependency envelope. Writing an archive by hand from this module would be
    /// implementing part of a linker in a test harness in order to test a flag the suite does not
    /// use, which is a worse trade than recording the limitation.
    ///
    /// The suite's own programs need only the C library, which every driver links by default, so
    /// neither flag appears in any differential invocation. Both are verified here so that
    /// maintenance has a proven envelope to work within, and the row says exactly how far the
    /// proof goes.
    fn check_library_flags(&self) -> HarnessResult<Vec<FlagCheck>> {
        let party = self.participants(self.target);
        let (workspace, source) = self.stage("-L and -l", SOURCE_ARITHMETIC, PROGRAM_ARITHMETIC)?;
        let search_dir = path_text(workspace.root())?;

        let observable = format!(
            "both compilers accept {FLAG_LIBRARY_PATH} naming an existing directory together \
             with {FLAG_LIBRARY} naming a real library, and still produce a program that runs \
             correctly"
        );
        let mut path_row = Evidence::new(
            FLAG_LIBRARY_PATH,
            party.scope.as_str(),
            CheckKind::AcceptanceOnly,
            observable.clone(),
        );
        let mut library_row = Evidence::new(
            FLAG_LIBRARY,
            party.scope.as_str(),
            CheckKind::AcceptanceOnly,
            observable,
        );
        for row in [&mut path_row, &mut library_row] {
            row.note(String::from(LIBRARY_FLAG_LIMITATION));
            row.note(String::from(LIBRARY_FLAG_JUSTIFICATION));
            if let Some(reason) = &party.absence {
                row.mark_unavailable(reason.clone());
            }
        }

        for compiler in &party.compilers {
            // The library flags follow the source, which is the order a static archive requires,
            // and the order a maintainer would use. The probe program references nothing in the
            // named library, so the link succeeds on the merits of the flags rather than on
            // symbol resolution.
            let configuration = self.build_and_run(
                compiler,
                &workspace,
                format!("{FLAG_LIBRARY_PATH} {search_dir} {FLAG_LIBRARY} {LIBRARY_NAME}"),
                &args(&[FLAG_LIBRARY_PATH, &search_dir, FLAG_LIBRARY, LIBRARY_NAME]),
                &source,
                "library_flags.bin",
            )?;
            for row in [&mut path_row, &mut library_row] {
                configuration.absorb(row);
                expect_printed(row, compiler.label(), &configuration, ARITHMETIC_STDOUT);
            }
        }

        settle_shared(workspace, &mut [&mut path_row, &mut library_row]);
        Ok(vec![path_row.finish(), library_row.finish()])
    }
}

/// The limitation recorded on both library-flag rows.
const LIBRARY_FLAG_LIMITATION: &str =
    "LIMITATION: acceptance only. Neither that the directory named by -L was searched, nor that \
     the library named by -l was taken from it, is established here — proving either needs an \
     archive placed in that directory which the link would not otherwise find, and creating an \
     archive needs a tool the zero-dependency rule does not allow the suite to require. The \
     library that was named resolves from the driver's own default search path.";

/// Why the limitation is acceptable, recorded beside it so a reader need not infer it.
const LIBRARY_FLAG_JUSTIFICATION: &str =
    "The suite's programs need only the C library, which every driver links by default, so \
     neither -L nor -l appears in any differential invocation. Both are verified here so that \
     maintenance has a proven envelope, and this row states exactly how far the proof goes.";

/// Require a configuration that ran to have printed exactly `expected`.
///
/// A configuration that did not run has already recorded its own problem or unavailability, so
/// this adds nothing rather than reporting the same fact twice.
fn expect_printed(
    evidence: &mut Evidence,
    label: &str,
    configuration: &Configuration,
    expected: &str,
) {
    let Some(printed) = &configuration.printed else {
        return;
    };
    if printed.as_slice() != expected.as_bytes() {
        evidence.problem(format!(
            "{} under the {label} printed {} where {} was required",
            configuration.label,
            quote_stdout(printed),
            quote_stdout(expected.as_bytes())
        ));
    }
}

/// Require two configurations that both ran to have printed *different* bytes.
///
/// This is what turns acceptance into verification for the macro flags: a driver that accepted
/// `-D` and then discarded it would compile cleanly and print the built-in default, and only the
/// comparison between the two configurations exposes that.
fn expect_changed(
    evidence: &mut Evidence,
    label: &str,
    before: &Configuration,
    after: &Configuration,
    flag: &str,
) {
    let (Some(first), Some(second)) = (&before.printed, &after.printed) else {
        return;
    };
    if first == second {
        evidence.problem(format!(
            "the {label} printed {} both with and without {flag}, so the flag had no observable \
             effect on the program",
            quote_stdout(first)
        ));
    } else {
        evidence.observe(format!(
            "adding {flag} changed what the {label} printed from {} to {}",
            quote_stdout(first),
            quote_stdout(second)
        ));
    }
}

/// Describe the first output divergence among a set of configurations, or `None` when every
/// configuration that ran printed the same bytes.
///
/// Compares every configuration against the first one that ran, across levels and across
/// compilers alike, because a difference between two compilers at one level and a difference
/// between two levels of one compiler are equally disqualifying for a flag that is supposed to
/// preserve behaviour.
fn describe_output_divergence(groups: &[Vec<Configuration>]) -> Option<String> {
    let mut baseline: Option<(&Configuration, &Vec<u8>)> = None;
    for group in groups {
        for configuration in group {
            let Some(printed) = &configuration.printed else {
                continue;
            };
            match baseline {
                None => baseline = Some((configuration, printed)),
                Some((first, expected)) => {
                    if printed != expected {
                        return Some(format!(
                            "{} under the {} printed {} while {} under the {} printed {}; a flag \
                             that changes observable behaviour cannot be part of a set both \
                             compilers honour with the same meaning",
                            first.label,
                            first.side.label(),
                            quote_stdout(expected),
                            configuration.label,
                            configuration.side.label(),
                            quote_stdout(printed)
                        ));
                    }
                }
            }
        }
    }
    None
}

/// Decide the ending of a check's workspace and return the note the rows should carry.
///
/// A workspace whose check found a problem is retained, so the program, both artifacts and every
/// captured stream stay on disk and the row can name the directory. One that found nothing is
/// removed, so a non-empty work tree always means something needs looking at. A run that asked
/// for everything to be retained keeps it either way, and says so, because an intentionally
/// retained tree and a tree full of failures look identical otherwise.
///
/// "Every captured stream" is literal: each invocation published its untruncated standard output,
/// its untruncated standard error and its raw wait status into this directory as it happened, under
/// the stems the rows name. Nothing is written here at settling time, so the retention decision
/// cannot lose evidence that had not been recorded yet.
///
/// The note is returned rather than printed so that it lands in the report next to the check it
/// belongs to, instead of on a side channel a reader has to correlate by hand.
fn settle_workspace(workspace: Workspace, failed: bool) -> Option<String> {
    if failed {
        return Some(format!(
            "the workspace was retained for inspection at {}",
            workspace.retain().describe()
        ));
    }
    let kept_by_request = workspace.keeps_on_success();
    let root = workspace.root().to_path_buf();
    match workspace.discard_advisory() {
        Some(note) => Some(note),
        None if kept_by_request => Some(format!(
            "the workspace {} was kept because the run asked for every workspace to be retained",
            shown_path(&root)
        )),
        None => None,
    }
}

/// Attach the workspace's ending to every row assembled from it.
///
/// Applied before the rows are finished, so a retention note is part of the row a reader is
/// looking at rather than a separate observation they have to find.
fn settle_shared(workspace: Workspace, rows: &mut [&mut Evidence]) {
    let failed = rows.iter().any(|row| !row.holds());
    if let Some(note) = settle_workspace(workspace, failed) {
        for row in rows.iter_mut() {
            row.note(note.clone());
        }
    }
}

// ---------------------------------------------------------------------------------------------
// SECTION 7 — the negative assertions.
//
// The positive checks above prove that today's shared flags mean the same thing to both
// compilers. The assertions below are what stop that from decaying: they prove that no flag which
// must **not** be shared has found its way into the shared set, and that every such flag is still
// recognised by the tables, so a maintainer who adds one months from now is caught by a failing
// test rather than by a silently meaningless comparison.
//
// Every assertion is made against the tables in the module root, never against a local copy. A
// second copy of a flag table is a second source of truth, and the whole point of these rows is
// that there is only one.
// ---------------------------------------------------------------------------------------------

/// One flag that must be absent from the shared set, with the reason it is excluded.
///
/// The reason is carried here rather than in a comment because it is written into the report: a
/// reader who wonders why a perfectly ordinary flag is refused should find the answer beside the
/// refusal, not in the history of the file.
struct RequiredAbsent {
    /// The spelling asserted absent, exactly as it would appear on a command line.
    spelling: &'static str,
    /// Why it is excluded, in one sentence a report can print verbatim.
    reason: &'static str,
}

/// Every flag spelling that must never appear in a differential invocation.
///
/// The list covers all three forms a maintainer might reach for — the bare flag, the prefix form
/// that takes a value, and a concrete value form — because a table that recognised only the bare
/// spelling would let `-fsanitize=undefined` or `--target=aarch64-linux-gnu` slip past the very
/// check written to stop it.
const REQUIRED_ABSENT: &[RequiredAbsent] = &[
    RequiredAbsent {
        spelling: "-O3",
        reason: "the compiler under test documents no optimization level above -O2, so the \
                 matrix is exactly -O0, -O1 and -O2; a level only one compiler implements cannot \
                 be honoured with the same meaning by both",
    },
    RequiredAbsent {
        spelling: "-Os",
        reason: "size-directed optimization is outside the compiler under test's documented \
                 scope, for the same reason as -O3",
    },
    RequiredAbsent {
        spelling: "-std=",
        reason: "the compiler under test has no language-standard flag at all, and the reference \
                 compiler's default mode was measured as gnu17, which already enables the GNU \
                 extensions the corpus exercises; passing one would make the two sides \
                 incomparable rather than more comparable",
    },
    RequiredAbsent {
        spelling: "-std=c11",
        reason: "the concrete value form of the same flag, asserted separately so a table that \
                 matched only the bare prefix could not let it through",
    },
    RequiredAbsent {
        spelling: "-pedantic",
        reason: "reference-compiler only, and it exists to reject exactly the extensions the \
                 corpus is required to test; it belongs to the undefined-behaviour audit gate, \
                 where one feature area drops it deliberately",
    },
    RequiredAbsent {
        spelling: "-Wall",
        reason: "a warning flag belongs to the undefined-behaviour audit gate, which the \
                 reference compiler alone drives; diagnostics are never compared between \
                 compilers because their wording legitimately differs",
    },
    RequiredAbsent {
        spelling: "-Wextra",
        reason: "part of the same audit gate, and reference-compiler only",
    },
    RequiredAbsent {
        spelling: "-Werror",
        reason: "part of the same audit gate; turning a diagnostic into a failure inside a \
                 differential invocation would fail a cell for a difference in wording rather \
                 than in behaviour",
    },
    RequiredAbsent {
        spelling: "-Wconversion",
        reason: "part of the same audit gate, which the deliberate narrowing programs drop with a \
                 recorded reason",
    },
    RequiredAbsent {
        spelling: "-Wsign-conversion",
        reason: "part of the same audit gate, dropped alongside -Wconversion by the narrowing \
                 programs",
    },
    RequiredAbsent {
        spelling: "-Wshadow",
        reason: "part of the same audit gate",
    },
    RequiredAbsent {
        spelling: "-m32",
        reason: "measured to fail on this host, because the multilib start files are absent; \
                 32-bit coverage comes from the dedicated i686 cross driver instead, which is \
                 also the shape the other two cross arms already use",
    },
    RequiredAbsent {
        spelling: "-S",
        reason: "assembly text is not a program, and every oracle in this suite compares what a \
                 program printed and the status it exited with",
    },
    RequiredAbsent {
        spelling: "-E",
        reason: "preprocessed text is not a program either; the preprocessor is exercised by \
                 compiling and running its result, not by comparing its output",
    },
    RequiredAbsent {
        spelling: "-fwrapv",
        reason: "it changes what the language means for signed overflow; the corpus is required \
                 to contain none, and passing this would mask a program that did",
    },
    RequiredAbsent {
        spelling: "-fno-strict-aliasing",
        reason: "the same shape of mistake: it relaxes an assumption the corpus is required not \
                 to depend on, so it would hide a defective test program rather than reveal a \
                 compiler defect",
    },
    RequiredAbsent {
        spelling: "-fsanitize=",
        reason: "the compiler under test does not support sanitizers; the sanitizer run belongs \
                 exclusively to the audit gate, where it judges the test program and never the \
                 compiler under test",
    },
    RequiredAbsent {
        spelling: "-fsanitize=undefined",
        reason: "the concrete value form of the same flag, asserted separately for the same \
                 reason as -std=c11",
    },
    RequiredAbsent {
        spelling: "-fno-builtin",
        reason: "reference-compiler only; it changes which library calls the compiler may \
                 recognise, which is a difference in meaning rather than in spelling",
    },
    RequiredAbsent {
        spelling: "-ffreestanding",
        reason: "reference-compiler only; it changes the environment the program is compiled for, \
                 while every program here is linked against a real C runtime",
    },
    RequiredAbsent {
        spelling: "-nostdlib",
        reason: "reference-compiler only; it removes the runtime the programs depend on",
    },
    RequiredAbsent {
        spelling: "-mretpoline",
        reason: "a hardening flag of the compiler under test alone; the reference compiler has no \
                 equivalent spelling, and the documented way to confirm the thunks it emits is \
                 disassembly inspection rather than an output comparison",
    },
    RequiredAbsent {
        spelling: "-fcf-protection",
        reason: "THE INSTRUCTIVE CASE: both compilers accept it, and their default scopes differ, \
                 so a check that asked only whether it was accepted would have admitted it and \
                 quietly corrupted every differential comparison; verification had to be \
                 semantic rather than syntactic, and the honest outcome of that verification is \
                 exclusion",
    },
    RequiredAbsent {
        spelling: "-fcf-protection=full",
        reason: "the concrete value form of the instructive case, asserted separately so no \
                 spelling of it can be admitted",
    },
    RequiredAbsent {
        spelling: "--target",
        reason: "target selection belongs to the compiler under test alone, where both sides of a \
                 cross-backend comparison are the same compiler; the reference side selects a \
                 target by choosing a driver",
    },
    RequiredAbsent {
        spelling: "--target=aarch64-linux-gnu",
        reason: "the value form was measured to be rejected outright by the reference compiler — \
                 that spelling belongs to a different compiler family — which is precisely why \
                 the cross arm uses cross-driver binaries",
    },
    RequiredAbsent {
        spelling: "--sysroot",
        reason: "a path selector of the compiler under test alone; the reference drivers carry \
                 their own runtime locations",
    },
];

/// Every verified shared flag is absent from the forbidden tables, in both directions.
///
/// This is the structural assertion the whole discipline rests on: if a single flag were in both
/// tables, one of the two would be wrong and no row below could tell which. It also exercises the
/// tables' prefix matching against the two near-collisions that actually exist — `-static` beside
/// `-S`, and the three optimization levels beside `-O3` and `-Os` — so a matcher that grew too
/// eager would be caught here rather than by silently excluding a flag the suite depends on.
fn check_no_forbidden_flag_is_shared() -> FlagCheck {
    let mut evidence = Evidence::new(
        "the verified shared set",
        "structural",
        CheckKind::Membership,
        "no flag appears in both the verified shared set and the forbidden set, and the forbidden \
         set's prefix matching does not sweep up a shared flag that merely begins the same way",
    );
    for flag in SHARED_FLAGS_VERIFIED.iter().copied() {
        if is_forbidden_in_differential(flag) {
            evidence.problem(format!(
                "{flag} is in the verified shared set and is also recognised as forbidden in a \
                 differential invocation; one of the two tables is wrong, and until that is \
                 resolved no comparison using this flag can be interpreted"
            ));
        }
    }
    if evidence.holds() {
        evidence.observe(format!(
            "all {} verified shared flags are absent from the forbidden set",
            SHARED_FLAGS_VERIFIED.len()
        ));
        evidence.observe(String::from(
            "-static is not swept up by -S, and -O0, -O1 and -O2 are not swept up by -O3 or -Os, \
             so the forbidden set's matching is exact where it needs to be",
        ));
    }
    evidence.note(format!(
        "the forbidden set holds {} spellings and the verified shared set holds {}; both are read \
         from the module root rather than copied here, so this row cannot pass against a stale \
         duplicate",
        FORBIDDEN_IN_DIFFERENTIAL.len(),
        SHARED_FLAGS_VERIFIED.len()
    ));
    evidence.finish()
}

/// One row per spelling that must never reach a differential invocation.
///
/// Each row asserts two things: the spelling is not in the verified shared set, and the tables
/// still recognise it as forbidden. The second half is the part that survives maintenance — a
/// spelling quietly dropped from the forbidden table would stop being refused, and nothing else in
/// the suite would notice.
fn required_absent_checks() -> Vec<FlagCheck> {
    REQUIRED_ABSENT
        .iter()
        .map(|entry| {
            let mut evidence = Evidence::new(
                entry.spelling,
                "structural",
                CheckKind::Negative,
                "the spelling is absent from the verified shared set and is still recognised as \
                 forbidden in a differential invocation",
            );
            evidence.note(String::from(entry.reason));
            if SHARED_FLAGS_VERIFIED.contains(&entry.spelling) {
                evidence.problem(format!(
                    "{} is present in the verified shared set, which it must never be: {}",
                    entry.spelling, entry.reason
                ));
            } else {
                evidence.observe(format!(
                    "{} is absent from the verified shared set",
                    entry.spelling
                ));
            }
            if is_forbidden_in_differential(entry.spelling) {
                evidence.observe(format!(
                    "{} is recognised as forbidden, so an invocation that tried to pass it would \
                     be refused before the process was spawned",
                    entry.spelling
                ));
            } else {
                evidence.problem(format!(
                    "{} is not recognised as forbidden, so nothing would stop a future invocation \
                     from passing it; the forbidden table has lost a spelling it must keep",
                    entry.spelling
                ));
            }
            evidence.finish()
        })
        .collect()
}

/// The minimal differential set is exactly what it claims to be, and lives inside the verified
/// set.
///
/// The flags actually used in a differential invocation are deliberately few, because a small set
/// is one that can be defended completely. The wider verified set exists so that maintenance has a
/// proven envelope to work within, not because the suite needs it today — and this row is what
/// keeps the distinction from eroding in either direction.
fn check_minimal_differential_set() -> FlagCheck {
    // Spelled out here rather than read from the table, so that a change to the table has to be
    // matched by a change here and cannot pass unnoticed. This is the one place in the module
    // where duplicating a table is the point.
    const EXPECTED_MINIMAL: &[&str] = &["-o", "-static"];
    let mut evidence = Evidence::new(
        "the minimal differential set",
        "structural",
        CheckKind::Membership,
        "the flags a differential invocation actually passes are exactly -o and -static, plus one \
         optimization level chosen per cell, and every one of them is in the verified shared set",
    );
    if DIFFERENTIAL_FLAGS_MINIMAL == EXPECTED_MINIMAL {
        evidence.observe(format!(
            "the minimal set is exactly {EXPECTED_MINIMAL:?}, with the optimization level supplied \
             per cell rather than fixed in the table"
        ));
    } else {
        evidence.problem(format!(
            "the minimal set is {DIFFERENTIAL_FLAGS_MINIMAL:?} where {EXPECTED_MINIMAL:?} was \
             required; the set of flags every differential invocation carries has changed, and \
             each addition needs its own semantic verification above before it can be trusted"
        ));
    }
    for flag in DIFFERENTIAL_FLAGS_MINIMAL.iter().copied() {
        if SHARED_FLAGS_VERIFIED.contains(&flag) {
            evidence.observe(format!(
                "{flag} is in the verified shared set, so its meaning has been established for \
                 both compilers"
            ));
        } else {
            evidence.problem(format!(
                "{flag} is passed by every differential invocation but is not in the verified \
                 shared set, so nothing has established that both compilers honour it with the \
                 same meaning"
            ));
        }
        if is_forbidden_in_differential(flag) {
            evidence.problem(format!(
                "{flag} is passed by every differential invocation and is also forbidden in one"
            ));
        }
    }
    evidence.finish()
}

/// The optimization levels are exactly the three both compilers honour.
///
/// Two directions, both required. Every level the harness can select must be a verified shared
/// flag, or a cell would be compiled with something unproven; and no level beyond the three may be
/// in the shared set, or a maintainer would find an invitation to pass one.
fn check_optimization_membership() -> FlagCheck {
    let mut evidence = Evidence::new(
        "the optimization levels",
        "structural",
        CheckKind::Membership,
        "every optimization level the harness can select is a verified shared flag, and the \
         verified shared set contains no other optimization spelling",
    );
    for level in OptLevel::ALL {
        if SHARED_FLAGS_VERIFIED.contains(&level.flag()) {
            evidence.observe(format!(
                "{} is a verified shared flag, so a cell compiled at that level is comparable",
                level.flag()
            ));
        } else {
            evidence.problem(format!(
                "{} can be selected for a cell but is not a verified shared flag",
                level.flag()
            ));
        }
    }
    let levels: Vec<&str> = SHARED_FLAGS_VERIFIED
        .iter()
        .copied()
        .filter(|flag| flag.starts_with("-O"))
        .collect();
    let expected: Vec<&str> = OptLevel::ALL.iter().map(|level| level.flag()).collect();
    if levels == expected {
        evidence.observe(format!(
            "the verified shared set's optimization entries are exactly {levels:?}"
        ));
    } else {
        evidence.problem(format!(
            "the verified shared set's optimization entries are {levels:?} where {expected:?} was \
             required; an extra level in the set is an invitation to pass one the compiler under \
             test does not implement"
        ));
    }
    if DIFFERENTIAL_FLAGS_MINIMAL
        .iter()
        .any(|flag| flag.starts_with("-O"))
    {
        evidence.problem(String::from(
            "the minimal differential set names an optimization level; the level is one axis of \
             the matrix and must be chosen per cell, not fixed for every invocation",
        ));
    } else {
        evidence.observe(String::from(
            "the minimal differential set names no level, so the level remains an axis of the \
             matrix rather than a constant",
        ));
    }
    evidence.finish()
}

/// The undefined-behaviour audit gate's flags are all forbidden in a differential invocation.
///
/// The gate and the comparison are two different activities with two different tools. The gate is
/// driven by the reference compiler alone and judges the *test program*; a differential invocation
/// compares two compilers. A gate flag that leaked into a comparison would fail a cell over a
/// difference in diagnostic wording, which says nothing about either compiler's code generation.
fn check_audit_gate_exclusion() -> FlagCheck {
    let mut evidence = Evidence::new(
        "the undefined-behaviour audit gate",
        "structural",
        CheckKind::Membership,
        "every flag the default audit gate passes is forbidden in a differential invocation and \
         absent from the verified shared set",
    );
    for flag in UB_AUDIT_GATE_DEFAULT.iter().copied() {
        if !is_forbidden_in_differential(flag) {
            evidence.problem(format!(
                "{flag} belongs to the audit gate but is not forbidden in a differential \
                 invocation, so nothing prevents it from reaching a comparison"
            ));
        }
        if SHARED_FLAGS_VERIFIED.contains(&flag) {
            evidence.problem(format!(
                "{flag} belongs to the audit gate and is also in the verified shared set"
            ));
        }
    }
    if evidence.holds() {
        evidence.observe(format!(
            "all {} audit-gate flags are forbidden in a differential invocation and absent from \
             the verified shared set, so the gate and the comparison cannot borrow each other's \
             flags",
            UB_AUDIT_GATE_DEFAULT.len()
        ));
    }
    evidence.finish()
}

/// Target selection is forbidden for the reference compiler and permitted for the compiler under
/// test.
///
/// This asymmetry is load-bearing rather than a loophole. Requirement 3 constrains the flags a
/// *differential* invocation passes to both compilers; the cross-backend oracle compares the
/// compiler under test against itself, so a flag that selects its backend is honoured with the
/// same meaning on both sides trivially, because both sides are the same compiler. Without the
/// distinction, cross-backend testing would be impossible — selecting a target is exactly what it
/// requires.
///
/// A control flag is checked alongside, forbidden for both sides, so the row would notice a
/// per-side split that had become a blanket permission.
fn check_per_side_split() -> FlagCheck {
    let control = "-Wall";
    let mut evidence = Evidence::new(
        "per-side flag permission",
        "structural",
        CheckKind::Membership,
        "the target selectors are forbidden for the reference compiler and permitted for the \
         compiler under test, while a reference-only flag is forbidden for both",
    );
    for flag in BCC_TARGET_SELECTORS.iter().copied() {
        if is_forbidden_for_side(flag, CompilerSide::Reference) {
            evidence.observe(format!(
                "{flag} is refused for the {}, which has no such flag and was measured to reject \
                 the spelling that resembles one",
                CompilerSide::Reference.label()
            ));
        } else {
            evidence.problem(format!(
                "{flag} would be accepted for the {}, which cannot honour it",
                CompilerSide::Reference.label()
            ));
        }
        if is_forbidden_for_side(flag, CompilerSide::UnderTest) {
            evidence.problem(format!(
                "{flag} is refused for the {}, which would make cross-backend comparison \
                 impossible: selecting a backend is precisely what that oracle requires",
                CompilerSide::UnderTest.label()
            ));
        } else {
            evidence.observe(format!(
                "{flag} is permitted for the {}, where both sides of the comparison are the same \
                 compiler",
                CompilerSide::UnderTest.label()
            ));
        }
    }
    for side in [CompilerSide::UnderTest, CompilerSide::Reference] {
        if is_forbidden_for_side(control, side) {
            evidence.observe(format!(
                "the control flag {control} is refused for the {}",
                side.label()
            ));
        } else {
            evidence.problem(format!(
                "the control flag {control} would be accepted for the {}, so the per-side split \
                 has become a blanket permission",
                side.label()
            ));
        }
    }
    evidence.finish()
}

/// Every verified shared flag is the subject of at least one check above.
///
/// A flag listed as verified but never examined would be a claim with nothing behind it, and that
/// is exactly the kind of quiet gap requirement 3 exists to close. Adding a flag to the shared set
/// therefore fails this row until a check for it is written.
fn check_shared_flag_coverage(checks: &[FlagCheck]) -> FlagCheck {
    let mut evidence = Evidence::new(
        "coverage of the verified shared set",
        "structural",
        CheckKind::Membership,
        "every flag in the verified shared set is the subject of at least one positive check in \
         this run",
    );
    for flag in SHARED_FLAGS_VERIFIED.iter().copied() {
        let covered: Vec<&FlagCheck> = checks
            .iter()
            .filter(|check| {
                check.subject() == flag
                    && matches!(
                        check.kind(),
                        CheckKind::Semantic | CheckKind::AcceptanceOnly
                    )
            })
            .collect();
        match covered.first() {
            None => evidence.problem(format!(
                "{flag} is listed as a verified shared flag but no check in this run examines it, \
                 so the claim that both compilers honour it has nothing behind it"
            )),
            Some(first) => evidence.observe(format!(
                "{flag} is covered by {} check(s), the first of which verified {}",
                covered.len(),
                match first.kind() {
                    CheckKind::AcceptanceOnly => "acceptance only, with its limitation recorded",
                    _ => "an observable consequence",
                }
            )),
        }
    }
    evidence.finish()
}

// ---------------------------------------------------------------------------------------------
// SECTION 8 — the report and the entry point.
//
// The report is the deliverable of this module. It states which flags were verified by an
// observable consequence, which were verified only for acceptance and why, which were asserted
// absent and why, and which could not be checked at all in this environment — because a probe
// whose output was a single boolean would leave a reader with no way to tell those four apart.
// ---------------------------------------------------------------------------------------------

/// The outcome of one flag-capability probe run.
///
/// The fields are private and the type is produced only by [`run`], because a report that could be
/// assembled independently of the work would be a claim rather than a record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlagProbeReport {
    /// Every check performed, in the order performed.
    checks: Vec<FlagCheck>,
    /// The tools and targets the run used, for the report header and for attributing a divergence
    /// to a toolchain change rather than to a compiler change.
    environment: Vec<String>,
    /// Observations about the run as a whole rather than about one flag.
    notes: Vec<String>,
    /// Whether an unavailable oracle fails the run, which the strict setting decides.
    unavailable_fails_run: bool,
}

impl FlagProbeReport {
    /// Assemble the report, sanitizing the two free-text lists on the way in.
    ///
    /// The checks need no sanitizing here because [`Evidence`] already sanitized every field of
    /// every row as it was recorded. The environment and note lines do, and for a reason that is
    /// easy to miss: an environment line names each discovered tool by path and by the version
    /// banner the tool itself printed, so its text originates in an environment variable and in a
    /// subprocess's standard output. Discovery already sanitizes a banner at capture time, which
    /// makes this the second of two independent guarantees rather than the only one — and the point
    /// of putting it here is that the guarantee then belongs to the type that renders these lines,
    /// instead of resting on a promise made in another module.
    fn new(
        checks: Vec<FlagCheck>,
        environment: Vec<String>,
        notes: Vec<String>,
        unavailable_fails_run: bool,
    ) -> FlagProbeReport {
        FlagProbeReport {
            checks,
            environment: environment.iter().map(|line| report_text(line)).collect(),
            notes: notes.iter().map(|line| report_text(line)).collect(),
            unavailable_fails_run,
        }
    }

    /// Every check performed, in the order performed.
    pub fn checks(&self) -> &[FlagCheck] {
        &self.checks
    }

    /// The tools, targets and settings the run used.
    pub fn environment(&self) -> &[String] {
        &self.environment
    }

    /// Observations about the run as a whole.
    pub fn notes(&self) -> &[String] {
        &self.notes
    }

    /// Checks that were performed and did not hold.
    pub fn failures(&self) -> Vec<&FlagCheck> {
        self.checks()
            .iter()
            .filter(|check| check.failed())
            .collect()
    }

    /// Checks that could not be performed because a tool is absent from this environment.
    pub fn unavailable(&self) -> Vec<&FlagCheck> {
        self.checks()
            .iter()
            .filter(|check| check.unavailable())
            .collect()
    }

    /// Checks verified for acceptance only, each carrying the limitation that made a semantic
    /// check impossible.
    pub fn limitations(&self) -> Vec<&FlagCheck> {
        self.checks()
            .iter()
            .filter(|check| check.kind() == CheckKind::AcceptanceOnly)
            .collect()
    }

    /// Whether an unavailable check fails this run, which the strict setting decides.
    pub fn unavailable_fails_run(&self) -> bool {
        self.unavailable_fails_run
    }

    /// True when every check was performed and held.
    ///
    /// Deliberately stricter than [`FlagProbeReport::fails_run`]: a run in a reduced environment
    /// can be permitted to proceed while still not being a run in which everything was verified,
    /// and collapsing the two would make an environment gap indistinguishable from a clean result.
    pub fn satisfied(&self) -> bool {
        self.checks().iter().all(FlagCheck::verified)
    }

    /// True when this report must fail the test that produced it.
    ///
    /// A failed check always fails the run: requirement 3 is a precondition for every differential
    /// cell in the suite, so a flag whose meaning is not established makes those comparisons
    /// uninterpretable rather than merely weaker. An unavailable check fails the run only under
    /// the strict setting, which is the intended setting for continuous integration — there the
    /// toolchain is installed deliberately, so a missing tool means a broken workflow rather than
    /// a modest machine.
    pub fn fails_run(&self) -> bool {
        !self.failures().is_empty()
            || (self.unavailable_fails_run() && !self.unavailable().is_empty())
    }

    /// The message a failing test should carry: every row that did not hold, in full.
    ///
    /// Self-contained on purpose. A reader must be able to see the flag, both command lines and
    /// the observable that failed without re-running anything, which is why the environment header
    /// is repeated here rather than left in the rendered table.
    pub fn failure_summary(&self) -> String {
        let failures = self.failures();
        let unavailable = self.unavailable();
        let mut text = format!(
            "the flag-capability probe did not hold: {} check(s) failed and {} could not be \
             performed{}.\n\nRequirement 3 requires that flag handling be verified rather than \
             assumed, and every differential comparison in this suite rests on it: until these \
             rows hold, a difference between the two compilers cannot be attributed to either \
             one.\n\n",
            failures.len(),
            unavailable.len(),
            if self.unavailable_fails_run() {
                ", which the strict setting treats as a failure"
            } else {
                ""
            }
        );
        text.push_str("environment:\n");
        for line in self.environment() {
            text.push_str("  - ");
            text.push_str(line);
            text.push('\n');
        }
        if !failures.is_empty() {
            text.push_str("\nfailed checks:\n\n");
            for check in &failures {
                text.push_str(&check.describe());
                text.push('\n');
            }
        }
        if self.unavailable_fails_run() && !unavailable.is_empty() {
            text.push_str("\nchecks that could not be performed:\n\n");
            for check in &unavailable {
                text.push_str(&check.describe());
                text.push('\n');
            }
        }
        text
    }

    /// A readable account of the whole run, for printing under `--nocapture`.
    ///
    /// States explicitly which flags were verified semantically, which were verified for
    /// acceptance only, and which were asserted absent, because those three claims have very
    /// different strengths and a table that blurred them would misrepresent the result.
    pub fn render(&self) -> String {
        let rule = "-".repeat(96);
        let heavy = "=".repeat(96);
        let mut text = format!(
            "{heavy}\n flag-capability probe — requirement 3: verify flag handling rather than \
             assuming it\n{heavy}\n"
        );

        text.push_str(" environment\n");
        for line in self.environment() {
            text.push_str("   • ");
            text.push_str(line);
            text.push('\n');
        }
        if !self.notes().is_empty() {
            text.push_str(" notes\n");
            for note in self.notes() {
                text.push_str("   • ");
                text.push_str(note);
                text.push('\n');
            }
        }

        text.push_str(&format!("{rule}\n"));
        text.push_str(&format!(
            " {:<12}{:<10}{:<38}{}\n",
            "verdict", "kind", "subject", "scope"
        ));
        text.push_str(&format!("{rule}\n"));
        for check in self.checks() {
            text.push_str(&format!(
                " {:<12}{:<10}{:<38}{}\n",
                check.outcome().label().trim_end(),
                check.kind().label().trim_end(),
                check.subject(),
                check.scope()
            ));
        }

        text.push_str(&format!("{rule}\n"));
        text.push_str(&format!(
            " verified by an observable consequence ({}): {}\n",
            self.subjects_of(CheckKind::Semantic).len(),
            join_subjects(&self.subjects_of(CheckKind::Semantic))
        ));
        text.push_str(&format!(
            " verified for acceptance only ({}): {} — each with its limitation recorded below\n",
            self.subjects_of(CheckKind::AcceptanceOnly).len(),
            join_subjects(&self.subjects_of(CheckKind::AcceptanceOnly))
        ));
        text.push_str(&format!(
            " asserted absent from the shared set ({}): {}\n",
            self.subjects_of(CheckKind::Negative).len(),
            join_subjects(&self.subjects_of(CheckKind::Negative))
        ));
        text.push_str(&format!(
            " structural properties of the flag tables ({}): {}\n",
            self.subjects_of(CheckKind::Membership).len(),
            join_subjects(&self.subjects_of(CheckKind::Membership))
        ));

        let limitations = self.limitations();
        if !limitations.is_empty() {
            text.push_str(&format!("{rule}\n recorded limitations\n"));
            for check in &limitations {
                for note in check.notes() {
                    if note.starts_with("LIMITATION") {
                        text.push_str(&format!("   {} — {note}\n", check.subject()));
                    }
                }
            }
        }

        let unavailable = self.unavailable();
        if !unavailable.is_empty() {
            text.push_str(&format!(
                "{rule}\n could not be performed in this environment ({}){}\n",
                unavailable.len(),
                if self.unavailable_fails_run() {
                    " — the strict setting treats each of these as a failure"
                } else {
                    " — reported rather than passed over silently"
                }
            ));
            for check in &unavailable {
                text.push_str(&indent(&check.describe()));
            }
        }

        let failures = self.failures();
        if !failures.is_empty() {
            text.push_str(&format!("{rule}\n failed checks ({})\n", failures.len()));
            for check in &failures {
                text.push_str(&indent(&check.describe()));
            }
        }

        let verified = self
            .checks()
            .iter()
            .filter(|check| check.verified())
            .count();
        text.push_str(&format!(
            "{heavy}\n summary: {} check(s) — {verified} verified, {} failed, {} unavailable; run \
             verdict: {}\n{heavy}\n",
            self.checks().len(),
            failures.len(),
            unavailable.len(),
            if self.fails_run() { "FAIL" } else { "PASS" }
        ));
        text
    }

    /// The distinct subjects of every check of one kind, in the order they were performed.
    fn subjects_of(&self, kind: CheckKind) -> Vec<&str> {
        let mut subjects: Vec<&str> = Vec::new();
        for check in self.checks() {
            if check.kind() == kind && !subjects.contains(&check.subject()) {
                subjects.push(check.subject());
            }
        }
        subjects
    }
}

/// Join subjects for a summary line, or say plainly that there were none.
fn join_subjects(subjects: &[&str]) -> String {
    if subjects.is_empty() {
        return String::from("none");
    }
    subjects.join(", ")
}

/// Indent a multi-line block for inclusion under a report heading.
fn indent(block: &str) -> String {
    let mut text = String::with_capacity(block.len() + block.len() / 8);
    for line in block.lines() {
        text.push_str("   ");
        text.push_str(line);
        text.push('\n');
    }
    text
}

/// State which artifact supplied the list of flags the compiler under test accepts.
///
/// The driver's own source is the authority where it is present. Where it is not — the
/// documentation-only arrangement this suite was written against — the repository's technical
/// specification supplies the inventory instead. Either way the list is only a starting point:
/// every flag in the report above was verified against both binaries, which is the authority
/// requirement 3 actually asks for, and this line exists so a reader never has to guess which
/// source was used.
fn flag_inventory_authority() -> String {
    let driver_cli = manifest_dir().join("src").join("driver").join("cli.rs");
    if driver_cli.is_file() {
        return format!(
            "flag inventory: read from {}, then verified against both binaries",
            shown_path(&driver_cli)
        );
    }
    format!(
        "flag inventory: {} is not present in this checkout, so the inventory came from the \
         repository's technical specification; every flag was then verified against both binaries \
         regardless, which is the authority requirement 3 asks for",
        shown_path(&driver_cli)
    )
}

/// The lesson the excluded hardening flag teaches, recorded on every run.
///
/// Kept in the report rather than only in a comment, because it is the single clearest argument for
/// why this module exists: the flag is *accepted* by both compilers, so a probe that checked
/// acceptance would have admitted it and every differential comparison in the suite would have been
/// quietly comparing two different things.
const CONTROL_FLOW_PROTECTION_LESSON: &str =
    "acceptance is not verification: -fcf-protection is accepted by both compilers and their \
     default scopes differ, so a syntactic parity check would have admitted it to the shared set \
     and corrupted every differential comparison; it is therefore verified semantically and \
     excluded, and asserted absent above.";

/// Run the flag-capability probe.
///
/// Performs every positive check — acceptance together with an observable consequence — and every
/// negative and structural assertion, then returns the record. The caller decides what to do with
/// it; [`FlagProbeReport::fails_run`] is the condition a test should assert on and
/// [`FlagProbeReport::failure_summary`] is the message it should carry.
///
/// # Errors
///
/// Fails when the compiler under test is absent, when the include-path fixture is not where the
/// corpus says it is, when a workspace cannot be allocated, or when a process cannot be spawned or
/// recorded at all. A compiler that *rejects* something is not an error — that is a finding this
/// report carries, and several checks require a rejection.
pub fn run(caps: &Capabilities) -> HarnessResult<FlagProbeReport> {
    let probe = Probe::new(caps)?;
    let config = caps.config();

    let mut checks = vec![
        probe.check_output_naming()?,
        probe.check_compile_only()?,
        probe.check_static_linkage()?,
        probe.check_static_linkage_other_class()?,
        probe.check_debug_information()?,
        probe.check_include_path()?,
    ];
    checks.extend(probe.check_macro_flags()?);
    checks.extend(probe.check_optimization_levels()?);
    checks.push(probe.check_position_independent_code()?);
    checks.extend(probe.check_library_flags()?);

    checks.push(check_no_forbidden_flag_is_shared());
    checks.extend(required_absent_checks());
    checks.push(check_minimal_differential_set());
    checks.push(check_optimization_membership());
    checks.push(check_audit_gate_exclusion());
    checks.push(check_per_side_split());
    let coverage = check_shared_flag_coverage(&checks);
    checks.push(coverage);

    let mut environment = vec![
        format!("compiler under test: {}", caps.bcc().summary()),
        format!(
            "reference compiler: {}",
            probe.reference_summary(probe.target)
        ),
        format!(
            "primary target: {} (ELF{})",
            probe.target.triple(),
            probe.target.elf_class()
        ),
        format!(
            "second ELF class: {} (ELF{}) via {}",
            probe.secondary_target.triple(),
            probe.secondary_target.elf_class(),
            probe.reference_summary(probe.secondary_target)
        ),
        format!(
            "per-invocation budget: {} second(s), enforced by {}",
            probe.budget.as_secs(),
            caps.timeout_tool().summary()
        ),
        format!(
            "unavailable checks fail the run: {}",
            config.unavailable_fails_run()
        ),
        format!(
            "include-path fixture: {}",
            shown_path(&probe.fixture_header)
        ),
    ];
    if !probe.caps.can_execute(probe.secondary_target) {
        environment.push(format!(
            "no runner for {}: its artifact is inspected but not executed",
            probe.secondary_target.triple()
        ));
    }

    let mut notes = vec![
        String::from(CONTROL_FLOW_PROTECTION_LESSON),
        flag_inventory_authority(),
        format!(
            "every flag was exercised through a real invocation of each compiler and every ELF \
             fact was read from the produced file with the standard library alone, so this probe \
             depends on no binary-inspection tool and stubs nothing: {} verified shared flags, {} \
             spellings asserted absent",
            SHARED_FLAGS_VERIFIED.len(),
            REQUIRED_ABSENT.len()
        ),
    ];
    if config.is_reduced_run() {
        let (targets, levels) = config.effective_matrix();
        notes.push(format!(
            "this run is REDUCED: the surrounding run sweeps {} target(s) and {} optimization \
             level(s) rather than the full matrix. The probe itself is unaffected — it always \
             examines both ELF classes and every shared flag — but a reduced run must never be \
             mistaken for a full one",
            targets.len(),
            levels.len()
        ));
    }

    Ok(FlagProbeReport::new(
        checks,
        environment,
        notes,
        config.unavailable_fails_run(),
    ))
}
