#!/usr/bin/env python3
"""Dump chained fixup structure from a Mach-O binary."""
import struct, sys

path = sys.argv[1] if len(sys.argv) > 1 else "fixtures/objc_foundation/app"
data = open(path, "rb").read()

# Find LC_DYLD_CHAINED_FIXUPS from load commands
# First, parse Mach-O header to find load commands
magic = struct.unpack_from("<I", data, 0)[0]
if magic == 0xFEEDFACF:  # MH_MAGIC_64
    ncmds = struct.unpack_from("<I", data, 16)[0]
    hdr_size = 32
elif magic == 0xFEEDFACE:
    ncmds = struct.unpack_from("<I", data, 16)[0]
    hdr_size = 28
else:
    print(f"Not a Mach-O: magic={magic:#x}")
    sys.exit(1)

off = hdr_size
fixup_off = fixup_size = 0
segments = []
for _ in range(ncmds):
    cmd = struct.unpack_from("<I", data, off)[0]
    cmdsize = struct.unpack_from("<I", data, off + 4)[0]
    if cmd == 0x80000034:  # LC_DYLD_CHAINED_FIXUPS
        fixup_off = struct.unpack_from("<I", data, off + 8)[0]
        fixup_size = struct.unpack_from("<I", data, off + 12)[0]
    if cmd == 0x19:  # LC_SEGMENT_64
        segname = data[off+8:off+24].split(b"\x00")[0].decode()
        vmaddr = struct.unpack_from("<Q", data, off + 24)[0]
        vmsize = struct.unpack_from("<Q", data, off + 32)[0]
        fileoff = struct.unpack_from("<Q", data, off + 40)[0]
        filesize = struct.unpack_from("<Q", data, off + 48)[0]
        segments.append((segname, vmaddr, vmsize, fileoff, filesize))
    off += cmdsize

print(f"LC_DYLD_CHAINED_FIXUPS: dataoff={fixup_off}, datasize={fixup_size}")
print(f"\nSegments:")
for i, (name, va, vs, fo, fs) in enumerate(segments):
    print(f"  [{i}] {name:16s} vmaddr={va:#x} vmsize={vs:#x} fileoff={fo:#x} filesize={fs:#x}")

buf = data[fixup_off:fixup_off + fixup_size]

# dyld_chained_fixups_header
fixups_version = struct.unpack_from("<I", buf, 0)[0]
starts_offset = struct.unpack_from("<I", buf, 4)[0]
imports_offset = struct.unpack_from("<I", buf, 8)[0]
symbols_offset = struct.unpack_from("<I", buf, 12)[0]
imports_count = struct.unpack_from("<I", buf, 16)[0]
imports_format = struct.unpack_from("<I", buf, 20)[0]
symbols_format = struct.unpack_from("<I", buf, 24)[0]

print(f"\nChained fixups header:")
print(f"  version: {fixups_version}")
print(f"  starts_offset: {starts_offset}")
print(f"  imports_offset: {imports_offset}")
print(f"  symbols_offset: {symbols_offset}")
print(f"  imports_count: {imports_count}")
print(f"  imports_format: {imports_format}")
print(f"  symbols_format: {symbols_format}")

# Parse imports
print(f"\nImports ({imports_count}):")
entry_size = {1: 4, 2: 8, 3: 16}.get(imports_format, 4)
for i in range(imports_count):
    pos = imports_offset + i * entry_size
    if imports_format == 3:
        lib = struct.unpack_from("<H", buf, pos)[0]
        flags = struct.unpack_from("<H", buf, pos + 2)[0]
        weak = flags & 1
        name_off = struct.unpack_from("<I", buf, pos + 4)[0]
    else:
        raw = struct.unpack_from("<I", buf, pos)[0]
        lib = raw & 0xFF
        weak = (raw >> 8) & 1
        name_off = raw >> 9
    
    # Read symbol name
    name_start = symbols_offset + name_off
    end = buf[name_start:].index(0) if 0 in buf[name_start:] else len(buf) - name_start
    name = buf[name_start:name_start + end].decode("utf-8", errors="replace")
    print(f"  [{i}] lib={lib} weak={weak} name='{name}'")

# Parse chain starts
si = starts_offset
seg_count = struct.unpack_from("<I", buf, si)[0]
print(f"\nChain starts (seg_count={seg_count}):")
formats = {1:"ARM64E", 2:"PTR_64", 6:"PTR_64_OFFSET", 12:"ARM64E_USERLAND", 13:"ARM64E_USERLAND24"}

for i in range(seg_count):
    seg_info_off = struct.unpack_from("<I", buf, si + 4 + i * 4)[0]
    seg_name = segments[i][0] if i < len(segments) else f"seg[{i}]"
    if seg_info_off == 0:
        print(f"  seg[{i}] ({seg_name}): no chains")
        continue
    
    a = si + seg_info_off
    sz = struct.unpack_from("<I", buf, a)[0]
    page_size = struct.unpack_from("<H", buf, a + 4)[0]
    ptr_format = struct.unpack_from("<H", buf, a + 6)[0]
    seg_offset = struct.unpack_from("<Q", buf, a + 8)[0]
    max_valid = struct.unpack_from("<I", buf, a + 16)[0]
    page_count = struct.unpack_from("<H", buf, a + 20)[0]
    
    fmt_name = formats.get(ptr_format, f"unknown({ptr_format})")
    print(f"  seg[{i}] ({seg_name}):")
    print(f"    size={sz} page_size={page_size:#x} ptr_format={ptr_format}({fmt_name})")
    print(f"    segment_offset={seg_offset:#x} max_valid={max_valid:#x} page_count={page_count}")
    
    # Walk the chain in raw file data to verify linking
    seg_vmaddr = segments[i][1] if i < len(segments) else 0
    seg_fileoff = segments[i][3] if i < len(segments) else 0
    
    for p in range(page_count):
        ps = struct.unpack_from("<H", buf, a + 22 + p * 2)[0]
        status = " (NONE)" if ps == 0xFFFF else ""
        print(f"    page_starts[{p}] = {ps:#x}{status}")
        
        if ps == 0xFFFF:
            continue
        
        # Walk the chain from file data
        chain_off = seg_fileoff + p * page_size + ps
        print(f"    Chain walk (file offset base={seg_fileoff:#x}, page offset={p * page_size + ps:#x}):")
        
        stride = {1: 8, 2: 4, 6: 4, 12: 8, 13: 8}.get(ptr_format, 4)
        cur = p * page_size + ps  # offset within segment
        step = 0
        while step < 50:  # safety limit
            foff = seg_fileoff + cur
            if foff + 8 > len(data):
                print(f"      [{step}] offset={cur:#x} OUT OF BOUNDS")
                break
            raw = struct.unpack_from("<Q", data, foff)[0]
            vmaddr_at = seg_vmaddr + cur
            
            if ptr_format in (2, 6):  # PTR_64 / PTR_64_OFFSET
                is_bind = (raw >> 63) & 1
                nxt = (raw >> 51) & 0xFFF
                if is_bind:
                    ordinal = raw & 0x00FFFFFF
                    print(f"      [{step}] seg_off={cur:#06x} vaddr={vmaddr_at:#x} BIND ord={ordinal} next={nxt} raw={raw:#018x}")
                else:
                    target = raw & 0x0FFFFFFFFF
                    high8 = (raw >> 36) & 0xFF
                    print(f"      [{step}] seg_off={cur:#06x} vaddr={vmaddr_at:#x} REBASE target={high8 << 56 | target:#x} next={nxt} raw={raw:#018x}")
            elif ptr_format in (1, 12, 13):
                is_auth = (raw >> 63) & 1
                is_bind = (raw >> 62) & 1
                if is_bind:
                    if ptr_format == 13:
                        ordinal = raw & 0x00FFFFFF
                        nxt = (raw >> 24) & 0x7FF
                    else:
                        ordinal = raw & 0xFFFF
                        if is_auth:
                            nxt = (raw >> 32) & 0x7FF
                        else:
                            nxt = (raw >> 51) & 0x7FF
                    print(f"      [{step}] seg_off={cur:#06x} vaddr={vmaddr_at:#x} BIND ord={ordinal} next={nxt} auth={is_auth} raw={raw:#018x}")
                else:
                    if is_auth:
                        nxt = (raw >> 32) & 0x7FF
                    else:
                        nxt = (raw >> 51) & 0x7FF
                    target = raw & 0x7FFFFFFFFFF if not is_auth else raw & 0xFFFFFFFF
                    print(f"      [{step}] seg_off={cur:#06x} vaddr={vmaddr_at:#x} REBASE next={nxt} auth={is_auth} raw={raw:#018x}")
            else:
                nxt = (raw >> 51) & 0xFFF
                print(f"      [{step}] seg_off={cur:#06x} vaddr={vmaddr_at:#x} raw={raw:#018x} next={nxt}")
            
            if nxt == 0:
                print(f"      (chain ends)")
                break
            cur += nxt * stride
            step += 1
