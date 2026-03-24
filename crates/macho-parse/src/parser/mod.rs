mod reader;
use reader::Reader;

use serde::{Deserialize, Serialize};

use crate::ParseError;
use macho_types::*;

use macho_types::load_command::{LoadCommandPayload, LoadCommandType};

/// Fully parsed Mach-O binary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedMachO {
    pub header: MachHeader,
    pub commands: Vec<LoadCommand>,
    pub segments: Vec<Segment>,
    pub sections: Vec<Section>,
    pub dylibs: Vec<DylibRef>,
    pub rpaths: Vec<String>,
    pub entry: Option<EntryPoint>,
    pub symbols: Option<SymbolTable>,
    pub uuid: Option<[u8; 16]>,
    /// Parsed code signature from LC_CODE_SIGNATURE (if present).
    pub code_signature: Option<CodeSignature>,
}

/// Parse a Mach-O binary from a byte slice.
pub fn parse(data: &[u8]) -> Result<ParsedMachO, ParseError> {
    let mut r = Reader::new(data);
    let header = r.read_header()?;
    let is_64 = header.is_64bit();

    let mut commands = Vec::with_capacity(header.num_commands as usize);
    let mut segments = Vec::new();
    let mut sections_all = Vec::new();
    let mut dylibs = Vec::new();
    let mut rpaths = Vec::new();
    let mut entry = None;
    let mut uuid = None;
    let mut symbols = None;
    let mut cs_off_size: Option<(u32, u32)> = None;

    for _ in 0..header.num_commands {
        let cmd_start = r.pos;
        let cmd_raw = r.read_u32()?;
        let cmd_size = r.read_u32()?;

        if cmd_size < 8 {
            return Err(ParseError::LoadCommandTooSmall {
                size: cmd_size,
                min: 8,
            });
        }

        let cmd_type = LoadCommandType::from(cmd_raw);
        let payload_end = cmd_start + cmd_size as usize;

        let payload = match cmd_type {
            LoadCommandType::Segment64 => {
                let seg = r.read_segment_64()?;
                for s in &seg.sections {
                    sections_all.push(s.clone());
                }
                segments.push(seg.clone());
                LoadCommandPayload::Segment(seg)
            }
            LoadCommandType::Segment => {
                let seg = r.read_segment_32()?;
                for s in &seg.sections {
                    sections_all.push(s.clone());
                }
                segments.push(seg.clone());
                LoadCommandPayload::Segment(seg)
            }
            LoadCommandType::Symtab => {
                let sym_off = r.read_u32()?;
                let nsyms = r.read_u32()?;
                let str_off = r.read_u32()?;
                let str_size = r.read_u32()?;

                // Parse the symbol table from raw data
                symbols = Some(parse_symbol_table(
                    data, sym_off, nsyms, str_off, str_size, is_64,
                )?);

                LoadCommandPayload::Symtab {
                    sym_off,
                    nsyms,
                    str_off,
                    str_size,
                }
            }
            LoadCommandType::LoadDylib
            | LoadCommandType::IdDylib
            | LoadCommandType::LoadWeakDylib
            | LoadCommandType::ReexportDylib
            | LoadCommandType::LazyLoadDylib
            | LoadCommandType::LoadUpwardDylib => {
                let dref = r.read_dylib_ref(cmd_start)?;
                // All dylib-loading commands contribute to the ordinal table
                // (LC_ID_DYLIB does not — it identifies this dylib itself).
                match cmd_type {
                    LoadCommandType::LoadDylib
                    | LoadCommandType::LoadWeakDylib
                    | LoadCommandType::ReexportDylib
                    | LoadCommandType::LazyLoadDylib
                    | LoadCommandType::LoadUpwardDylib => {
                        dylibs.push(dref.clone());
                    }
                    _ => {}
                }
                match cmd_type {
                    LoadCommandType::LoadDylib => LoadCommandPayload::LoadDylib(dref),
                    LoadCommandType::LoadWeakDylib => LoadCommandPayload::LoadWeakDylib(dref),
                    LoadCommandType::ReexportDylib => LoadCommandPayload::ReexportDylib(dref),
                    LoadCommandType::LazyLoadDylib => LoadCommandPayload::LazyLoadDylib(dref),
                    LoadCommandType::LoadUpwardDylib => LoadCommandPayload::LoadUpwardDylib(dref),
                    _ => LoadCommandPayload::IdDylib(dref),
                }
            }
            LoadCommandType::LoadDylinker => {
                let name = r.read_lc_string(cmd_start)?;
                LoadCommandPayload::LoadDylinker { name }
            }
            LoadCommandType::Rpath => {
                let path = r.read_lc_string(cmd_start)?;
                rpaths.push(path.clone());
                LoadCommandPayload::Rpath { path }
            }
            LoadCommandType::Main => {
                let entry_offset = r.read_u64()?;
                let stack_size = r.read_u64()?;
                entry = Some(EntryPoint::Main {
                    entry_offset,
                    stack_size,
                });
                LoadCommandPayload::Main {
                    entry_offset,
                    stack_size,
                }
            }
            LoadCommandType::Uuid => {
                let mut bytes = [0u8; 16];
                bytes.copy_from_slice(r.read_bytes(16)?);
                uuid = Some(bytes);
                LoadCommandPayload::Uuid { uuid: bytes }
            }
            LoadCommandType::BuildVersion => {
                let platform = r.read_u32()?;
                let minos = r.read_u32()?;
                let sdk = r.read_u32()?;
                let ntools = r.read_u32()?;
                LoadCommandPayload::BuildVersion {
                    platform,
                    minos,
                    sdk,
                    ntools,
                }
            }
            LoadCommandType::DyldInfo | LoadCommandType::DyldInfoOnly => {
                let rebase_off = r.read_u32()?;
                let rebase_size = r.read_u32()?;
                let bind_off = r.read_u32()?;
                let bind_size = r.read_u32()?;
                let weak_bind_off = r.read_u32()?;
                let weak_bind_size = r.read_u32()?;
                let lazy_bind_off = r.read_u32()?;
                let lazy_bind_size = r.read_u32()?;
                let export_off = r.read_u32()?;
                let export_size = r.read_u32()?;
                LoadCommandPayload::DyldInfo {
                    rebase_off,
                    rebase_size,
                    bind_off,
                    bind_size,
                    weak_bind_off,
                    weak_bind_size,
                    lazy_bind_off,
                    lazy_bind_size,
                    export_off,
                    export_size,
                }
            }
            LoadCommandType::CodeSignature
            | LoadCommandType::SegmentSplitInfo
            | LoadCommandType::FunctionStarts
            | LoadCommandType::DataInCode
            | LoadCommandType::DyldExportsTrie
            | LoadCommandType::DyldChainedFixups => {
                let data_off = r.read_u32()?;
                let data_size = r.read_u32()?;
                if cmd_type == LoadCommandType::CodeSignature {
                    cs_off_size = Some((data_off, data_size));
                }
                LoadCommandPayload::LinkEdit {
                    data_off,
                    data_size,
                }
            }
            LoadCommandType::EncryptionInfo | LoadCommandType::EncryptionInfo64 => {
                let crypt_off = r.read_u32()?;
                let crypt_size = r.read_u32()?;
                let crypt_id = r.read_u32()?;
                // EncryptionInfo64 has an extra u32 pad field
                if cmd_type == LoadCommandType::EncryptionInfo64 {
                    let _pad = r.read_u32()?;
                }
                LoadCommandPayload::EncryptionInfo {
                    crypt_off,
                    crypt_size,
                    crypt_id,
                }
            }
            LoadCommandType::UnixThread => {
                // LC_UNIXTHREAD contains a flavor + count + register state.
                // We extract the instruction pointer (RIP for x86_64, PC for arm64).
                let _flavor = r.read_u32()?;
                let count = r.read_u32()?;
                // Read the full register state
                let state_bytes = count as usize * 4;
                let state = r.read_bytes(state_bytes.min(payload_end.saturating_sub(r.pos)))?;
                // x86_64: RIP is at offset 16*8 = 128 bytes into the state
                // arm64:  PC is at offset 32*8 = 256 bytes into the state
                let entry_address = if is_64 && state.len() >= 136 {
                    // x86_64 thread state: RIP at offset 128
                    u64::from_le_bytes([
                        state[128], state[129], state[130], state[131], state[132], state[133],
                        state[134], state[135],
                    ])
                } else if state.len() >= 264 {
                    // arm64 thread state: PC at offset 256
                    u64::from_le_bytes([
                        state[256], state[257], state[258], state[259], state[260], state[261],
                        state[262], state[263],
                    ])
                } else if state.len() >= 44 {
                    // i386: EIP at offset 40
                    u32::from_le_bytes([state[40], state[41], state[42], state[43]]) as u64
                } else {
                    0
                };
                entry = Some(EntryPoint::UnixThread { entry_address });
                LoadCommandPayload::UnixThread { entry_address }
            }
            LoadCommandType::VersionMinMacOS
            | LoadCommandType::VersionMinIOS
            | LoadCommandType::VersionMinTvOS
            | LoadCommandType::VersionMinWatchOS => {
                let version = r.read_u32()?;
                let sdk = r.read_u32()?;
                LoadCommandPayload::VersionMin { version, sdk }
            }
            LoadCommandType::DyldEnvironment => {
                let name = r.read_lc_string(cmd_start)?;
                LoadCommandPayload::DyldEnvironment { name }
            }
            LoadCommandType::SourceVersion => {
                let version = r.read_u64()?;
                LoadCommandPayload::SourceVersion { version }
            }
            LoadCommandType::Dysymtab => {
                let ilocalsym = r.read_u32()?;
                let nlocalsym = r.read_u32()?;
                let iextdefsym = r.read_u32()?;
                let nextdefsym = r.read_u32()?;
                let iundefsym = r.read_u32()?;
                let nundefsym = r.read_u32()?;
                let _tocoff = r.read_u32()?;
                let _ntoc = r.read_u32()?;
                let _modtaboff = r.read_u32()?;
                let _nmodtab = r.read_u32()?;
                let _extrefsymoff = r.read_u32()?;
                let _nextrefsyms = r.read_u32()?;
                let indirectsymoff = r.read_u32()?;
                let nindirectsyms = r.read_u32()?;
                let _extreloff = r.read_u32()?;
                let _nextrel = r.read_u32()?;
                let _locreloff = r.read_u32()?;
                let _nlocrel = r.read_u32()?;
                LoadCommandPayload::Dysymtab {
                    ilocalsym,
                    nlocalsym,
                    iextdefsym,
                    nextdefsym,
                    iundefsym,
                    nundefsym,
                    indirectsymoff,
                    nindirectsyms,
                }
            }
            _ => {
                let remaining = payload_end.saturating_sub(r.pos);
                let raw = r.read_bytes(remaining)?.to_vec();
                LoadCommandPayload::Unknown(raw)
            }
        };

        // Advance reader to the end of this command regardless.
        r.pos = payload_end;

        commands.push(LoadCommand {
            cmd: cmd_type,
            cmd_size,
            payload,
        });
    }

    // Parse code signature if LC_CODE_SIGNATURE was found
    let code_signature = if let Some((cs_off, cs_size)) = cs_off_size {
        let cs_start = cs_off as usize;
        let cs_end = cs_start + cs_size as usize;
        if cs_end <= data.len() {
            match crate::codesig::parse_code_signature(&data[cs_start..cs_end]) {
                Ok(cs) => Some(cs),
                Err(e) => {
                    log::warn!("code signature parse failed: {e}");
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    Ok(ParsedMachO {
        header,
        commands,
        segments,
        sections: sections_all,
        dylibs,
        rpaths,
        entry,
        symbols,
        uuid,
        code_signature,
    })
}

fn parse_symbol_table(
    data: &[u8],
    sym_off: u32,
    nsyms: u32,
    str_off: u32,
    str_size: u32,
    is_64: bool,
) -> Result<SymbolTable, ParseError> {
    let entry_size = if is_64 { 16usize } else { 12usize };
    let sym_start = sym_off as usize;
    let sym_end = sym_start + (nsyms as usize) * entry_size;
    let str_start = str_off as usize;
    let str_end = str_start + str_size as usize;

    if sym_end > data.len() || str_end > data.len() {
        return Err(ParseError::UnexpectedEof {
            offset: sym_start,
            needed: sym_end - sym_start,
            available: data.len().saturating_sub(sym_start),
        });
    }

    let str_table = &data[str_start..str_end];
    let mut symbols = Vec::with_capacity(nsyms as usize);

    let mut r = Reader::new(&data[sym_start..sym_end]);
    for _ in 0..nsyms {
        let str_index = r.read_u32()? as usize;
        let n_type = r.read_u8()?;
        let n_sect = r.read_u8()?;
        let n_desc = r.read_u16()?;
        let n_value = if is_64 {
            r.read_u64()?
        } else {
            r.read_u32()? as u64
        };

        let name = if str_index < str_table.len() {
            let end = str_table[str_index..]
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(str_table.len() - str_index);
            String::from_utf8_lossy(&str_table[str_index..str_index + end]).into_owned()
        } else {
            String::new()
        };

        symbols.push(Symbol {
            name,
            sym_type: SymbolType::from_ntype(n_type),
            section_index: n_sect,
            desc: n_desc,
            value: n_value,
        });
    }

    Ok(SymbolTable { symbols })
}

// ── Byte reader ──────────────────────────────────────────────────────
