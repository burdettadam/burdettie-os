import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import importlib.util
import json
from pathlib import Path
import struct
import tempfile
import unittest

SCRIPT = Path(__file__).parents[1] / "inspect_puredarwin_dyld.py"
spec = importlib.util.spec_from_file_location("dyld_inspector", SCRIPT)
mod = importlib.util.module_from_spec(spec); spec.loader.exec_module(mod)

def cmd(kind, payload):
    size = 8 + len(payload)
    size += (-size) % 8
    return struct.pack("<II", kind, size) + payload + b"\0" * (size - 8 - len(payload))

def segment(name, vmaddr, vmsize, fileoff, filesize, prot, nsects=0):
    body = name.encode().ljust(16,b"\0") + struct.pack("<QQQQIIII", vmaddr,vmsize,fileoff,filesize,prot,prot,nsects,0)
    return cmd(0x19, body + b"" * (nsects*80))

def fixture(minimum=0x001a0500, sdk=0x001a0500, filetype=7, install=b"/usr/lib/dyld", entry=0x100001000, include_thread=True):
    ident = struct.pack("<I",12) + install + b"\0"
    build = struct.pack("<IIII",1,minimum,sdk,0)
    state = [0]*42; state[32] = entry & 0xffffffff; state[33] = (entry >> 32) & 0xffffffff
    thread = struct.pack("<II",4,42) + struct.pack("<42I",*state)
    commands = [cmd(0xf,ident),cmd(0x32,build),segment("__TEXT",0x100000000,0x2000,0,0x2000,5)]
    if include_thread: commands.append(cmd(5,thread))
    region=b"".join(commands)
    header=struct.pack("<IiiIIIII",0xfeedfacf,0x01000007,3,filetype,len(commands),len(region),0,0)
    image = header + region
    return image + b"\x90" * (0x2000 - len(image))

class DyldInspectorTests(unittest.TestCase):
    def setUp(self): self.expected={"minimum_os":"26.5.0","sdk_version":"26.5.0"}
    def test_valid_dylinker_structure(self):
        got=mod.parse(fixture(),self.expected)
        self.assertEqual(got["file_type"],"MH_DYLINKER")
        self.assertEqual(got["install_name"],"/usr/lib/dyld")
        self.assertEqual(got["entry_point"],0x100001000)
    def test_rejects_dylib_or_executable_file_type(self):
        for kind in (2,6,8):
            with self.subTest(kind=kind), self.assertRaises(mod.InspectionError): mod.parse(fixture(filetype=kind),self.expected)
    def test_rejects_wrong_install_name(self):
        with self.assertRaises(mod.InspectionError): mod.parse(fixture(install=b"/usr/lib/other"),self.expected)
    def test_rejects_wrong_build_version(self):
        with self.assertRaises(mod.InspectionError): mod.parse(fixture(minimum=0x001a0400),self.expected)
    def test_rejects_missing_thread_state(self):
        with self.assertRaises(mod.InspectionError): mod.parse(fixture(include_thread=False),self.expected)
    def test_rejects_entry_outside_executable_segment(self):
        with self.assertRaises(mod.InspectionError): mod.parse(fixture(entry=0x200000000),self.expected)
    def test_rejects_rpath_and_load_dylinker(self):
        base=fixture(); header=base[:32]; region=base[32:]
        for extra in (cmd(0x8000001c,struct.pack("<I",12)+b"/tmp\0"),cmd(0xe,struct.pack("<I",12)+b"/x\0")):
            n=struct.unpack_from("<I",header,16)[0]
            mutated=bytearray(header+region); mutated[16:20]=struct.pack("<I",n+1); mutated[20:24]=struct.pack("<I",len(region)+len(extra)); mutated[32:32]=extra
            with self.subTest(extra=extra), self.assertRaises(mod.InspectionError): mod.parse(bytes(mutated),self.expected)
    def test_rejects_unknown_load_command(self):
        base=fixture(); header=base[:32]; region=base[32:]
        extra=cmd(0x7ffffffe,b"\0"*8)
        n=struct.unpack_from("<I",header,16)[0]
        mutated=bytearray(header+region); mutated[16:20]=struct.pack("<I",n+1); mutated[20:24]=struct.pack("<I",len(region)+len(extra)); mutated[32:32]=extra
        with self.assertRaises(mod.InspectionError): mod.parse(bytes(mutated),self.expected)
    def test_rejects_truncated_header_and_load_commands(self):
        for raw in (b"\0"*20, fixture()[:40]):
            with self.assertRaises(mod.InspectionError): mod.parse(raw,self.expected)
    def test_rejects_changed_input_hash(self):
        with tempfile.TemporaryDirectory() as d:
            f=Path(d)/"dyld"; f.write_bytes(fixture())
            e=Path(d)/"expect.json"; value={"schema_version":1,"scope":"m1_dyld_executable_expectations","source_revision":mod.SOURCE_REVISION,"install_name":mod.INSTALL_NAME,"minimum_os":"26.5.0","sdk_version":"26.5.0"}; raw=json.dumps(value).encode(); e.write_bytes(raw)
            with self.assertRaises(mod.InspectionError): mod.inspect(f,"0"*64,e,mod.sha256(raw))
    def test_cli_exclusive_result_publication_and_prior_file_preservation(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d); f=root/"dyld"; f.write_bytes(fixture())
            e=root/"expect.json"; value={"schema_version":1,"scope":"m1_dyld_executable_expectations","source_revision":mod.SOURCE_REVISION,"install_name":mod.INSTALL_NAME,"minimum_os":"26.5.0","sdk_version":"26.5.0"}; raw=json.dumps(value).encode(); e.write_bytes(raw)
            out=root/"result.json"
            argv=["--dyld",str(f),"--dyld-sha256",mod.sha256(f.read_bytes()),"--expectations",str(e),"--expectations-sha256",mod.sha256(raw),"--output",str(out)]
            self.assertEqual(mod.main(argv),0)
            prior=out.read_bytes()
            self.assertEqual(mod.main(argv),1)
            self.assertEqual(out.read_bytes(),prior)
    def test_rejects_duplicate_expectation_json_keys(self):
        with tempfile.TemporaryDirectory() as d:
            path=Path(d)/"expect.json"
            raw=(b'{"schema_version":1,"scope":"m1_dyld_executable_expectations",'
                 b'"source_revision":"0163b2db225b85e3bf96fc25646ea290105ac298",'
                 b'"install_name":"/usr/lib/dyld","minimum_os":"26.5.0",'
                 b'"sdk_version":"26.5.0","sdk_version":"99.0.0"}')
            path.write_bytes(raw)
            with self.assertRaises(mod.InspectionError): mod.load_expectations(path,mod.sha256(raw))
    def test_rejects_symlink_input(self):
        with tempfile.TemporaryDirectory() as d:
            f=Path(d)/"real"; f.write_bytes(fixture()); link=Path(d)/"link"; link.symlink_to(f)
            with self.assertRaises(OSError): mod.read_regular(link,mod.MAX_INPUT)

if __name__=="__main__": unittest.main()
