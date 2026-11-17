import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import importlib.util
import shutil
import struct
import subprocess
import tempfile
import unittest
from pathlib import Path

P=Path(__file__).parents[1]/"inspect_puredarwin_compiler_rt.py"
spec=importlib.util.spec_from_file_location("compiler_rt",P); m=importlib.util.module_from_spec(spec); spec.loader.exec_module(m)

def ar_member(name,payload,mode=b"100644"):
    name=(name+"/").encode().ljust(16,b" ")
    fields=name+b"0".ljust(12)+b"0".ljust(6)+b"0".ljust(6)+mode.ljust(8)+str(len(payload)).encode().ljust(10)+b"`\n"
    assert len(fields)==60
    return fields+payload+(b"\n" if len(payload)&1 else b"")

def macho():
    return struct.pack("<IIIIIIII",0xFEEDFACF,m.CPU_X86_64,3,1,0,0,0,0)+b"\0"*16

class ArchiveTests(unittest.TestCase):
    def test_valid_single_x86_object(self):
        items=m.parse_ar(m.AR_MAGIC+ar_member("probe.o",macho()))
        self.assertEqual(items[0]["architecture"],"x86_64")
    def test_reject_thin(self):
        with self.assertRaisesRegex(m.InspectionError,"thin"):
            m.parse_ar(m.THIN_MAGIC)
    def test_reject_wrong_magic(self):
        with self.assertRaisesRegex(m.InspectionError,"magic"):
            m.parse_ar(b"not archive")
    def test_reject_non_object(self):
        b=bytearray(macho()); struct.pack_into("<I",b,12,6)
        with self.assertRaisesRegex(m.InspectionError,"MH_OBJECT"):
            m.parse_ar(m.AR_MAGIC+ar_member("bad.o",bytes(b)))
    def test_reject_big_endian_object(self):
        b=bytearray(macho()); b[:4]=b"\xfe\xed\xfa\xcf"
        with self.assertRaisesRegex(m.InspectionError,"non-little-endian"):
            m.parse_ar(m.AR_MAGIC+ar_member("bad.o",bytes(b)))
    def test_reject_wrong_arch(self):
        b=bytearray(macho()); struct.pack_into("<I",b,4,0x0100000c)
        with self.assertRaisesRegex(m.InspectionError,"x86_64"):
            m.parse_ar(m.AR_MAGIC+ar_member("bad.o",bytes(b)))
    def test_reject_bitcode(self):
        with self.assertRaisesRegex(m.InspectionError,"bitcode"):
            m.parse_ar(m.AR_MAGIC+ar_member("bitcode.o",b"BC\xc0\xde"+b"0"*8))
    def test_reject_unknown_member(self):
        with self.assertRaisesRegex(m.InspectionError,"unrecognized"):
            m.parse_ar(m.AR_MAGIC+ar_member("text.o",b"hello world"))
    def test_reject_truncated_header(self):
        with self.assertRaisesRegex(m.InspectionError,"truncated"):
            m.parse_ar(m.AR_MAGIC+b"x"*20)
    def test_reject_duplicate_names(self):
        with self.assertRaisesRegex(m.InspectionError,"duplicate"):
            m.parse_ar(m.AR_MAGIC+ar_member("same.o",macho())+ar_member("same.o",macho()))
    def test_reject_nonzero_odd_member_padding(self):
        archive=bytearray(m.AR_MAGIC+ar_member("odd.o",macho()+b"x"))
        archive[-1]=0
        with self.assertRaisesRegex(m.InspectionError,"odd-byte padding"):
            m.parse_ar(bytes(archive))
    def test_reject_invalid_mode_field(self):
        member=bytearray(ar_member("probe.o",macho()))
        member[40:48]=b"notmode "
        with self.assertRaisesRegex(m.InspectionError,"mode"):
            m.parse_ar(m.AR_MAGIC+bytes(member))
    def test_reject_duplicate_expectation_keys(self):
        raw=b'{"schema_version":1,"schema_version":1}'
        with self.assertRaisesRegex(m.InspectionError,"duplicate JSON key"):
            __import__("json").loads(raw,object_pairs_hook=m.unique_object)
    @unittest.skipUnless(shutil.which("clang") and shutil.which("llvm-ar"), "clang/llvm-ar unavailable")
    def test_real_llvm_ar_darwin_object_archive(self):
        with tempfile.TemporaryDirectory() as td:
            root=Path(td); source=root/"probe.c"; obj=root/"probe.o"; archive=root/"libprobe.a"
            source.write_text("int probe_div(int a, int b) { return a / b; }\n")
            subprocess.run(["clang","-target","x86_64-apple-macosx26.5","-ffreestanding","-c",str(source),"-o",str(obj)],check=True,capture_output=True)
            subprocess.run(["llvm-ar","rcs",str(archive),str(obj)],check=True,capture_output=True)
            members=m.parse_ar(archive.read_bytes())
            self.assertEqual(len(members),1)
            self.assertEqual(members[0]["architecture"],"x86_64")
            self.assertEqual(members[0]["kind"],"macho_object")
            self.assertIn("_probe_div",members[0]["defined_symbols"])
    def test_reject_member_overrun(self):
        h=b"bad.o/".ljust(16)+b"0".ljust(12)+b"0".ljust(6)+b"0".ljust(6)+b"100644".ljust(8)+b"999".ljust(10)+b"`\n"
        with self.assertRaisesRegex(m.InspectionError,"extends beyond"):
            m.parse_ar(m.AR_MAGIC+h+b"x")

if __name__=="__main__": unittest.main()
