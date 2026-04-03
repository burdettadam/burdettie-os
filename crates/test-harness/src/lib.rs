use std::path::Path;
use std::process::Command;

use macho_diff::{DiffReport, HostObservation, ObservedSegment};
use macho_fixtures::FixtureDescriptor;
use macho_parse::parse;
use macho_run_plan::ExecutionPlan;

/// Run the full comparison pipeline for a fixture.
pub fn run_fixture(
    descriptor: &FixtureDescriptor,
) -> Result<DiffReport, Box<dyn std::error::Error>> {
    let binary_path = &descriptor.binary_path;
    let data = std::fs::read(binary_path)?;
    let parsed = parse(&data)?;

    let plan = ExecutionPlan::from_parsed(&parsed, binary_path.clone());

    // Collect host observations.
    let observation = collect_host_observation(binary_path)?;

    // Compare planned dylibs with observed.
    let planned_dylibs: Vec<String> = plan
        .dependencies
        .graph
        .edges
        .iter()
        .map(|e| e.raw_name.clone())
        .collect();

    let report = DiffReport::compare_dylibs(&planned_dylibs, &observation.loaded_dylibs);

    Ok(report)
}

/// Run comparison with transitive dependency resolution.
pub fn run_fixture_transitive(
    descriptor: &FixtureDescriptor,
) -> Result<(DiffReport, ExecutionPlan, HostObservation), Box<dyn std::error::Error>> {
    let binary_path = &descriptor.binary_path;
    let data = std::fs::read(binary_path)?;
    let parsed = parse(&data)?;

    let plan = ExecutionPlan::from_parsed_transitive(&parsed, binary_path.clone());
    let observation = collect_host_observation(binary_path)?;

    let planned_dylibs: Vec<String> = plan
        .dependencies
        .graph
        .edges
        .iter()
        .map(|e| e.raw_name.clone())
        .collect();

    let report = DiffReport::compare_dylibs(&planned_dylibs, &observation.loaded_dylibs);

    Ok((report, plan, observation))
}

/// Collect host observation data for a binary using otool.
pub fn collect_host_observation(
    binary_path: &Path,
) -> Result<HostObservation, Box<dyn std::error::Error>> {
    // Get linked dylibs from otool -L
    let output = Command::new("otool")
        .args(["-L", &binary_path.to_string_lossy()])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let loaded_dylibs: Vec<String> = stdout
        .lines()
        .skip(1) // first line is the binary name
        .filter_map(|line| {
            let trimmed = line.trim();
            trimmed.split(" (").next().map(|s| s.to_string())
        })
        .collect();

    // Get segments and entry point from otool -l
    let output_l = Command::new("otool")
        .args(["-l", &binary_path.to_string_lossy()])
        .output()?;

    let stdout_l = String::from_utf8_lossy(&output_l.stdout);
    let (segments, entry_offset) = parse_otool_load_commands(&stdout_l);

    Ok(HostObservation {
        loaded_dylibs,
        segments,
        entry_offset,
    })
}

/// Parse `otool -l` output to extract segment info and entry point.
fn parse_otool_load_commands(output: &str) -> (Vec<ObservedSegment>, Option<u64>) {
    let mut segments = Vec::new();
    let mut entry_offset: Option<u64> = None;
    let mut lines = output.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();

        // Look for segment commands
        if trimmed == "cmd LC_SEGMENT_64" || trimmed == "cmd LC_SEGMENT" {
            let mut name = String::new();
            let mut vmaddr: u64 = 0;
            let mut vmsize: u64 = 0;

            // Read subsequent lines for this segment's fields
            while let Some(field_line) = lines.peek() {
                let ft = field_line.trim();
                if ft.starts_with("cmd ") || ft.starts_with("Load command") {
                    break;
                }
                let fl = lines.next().unwrap().trim().to_string();

                if fl.starts_with("segname ") {
                    name = fl.strip_prefix("segname ").unwrap_or("").trim().to_string();
                } else if fl.starts_with("vmaddr ") {
                    let val = fl.strip_prefix("vmaddr ").unwrap_or("0").trim();
                    vmaddr = u64::from_str_radix(val.trim_start_matches("0x"), 16).unwrap_or(0);
                } else if fl.starts_with("vmsize ") {
                    let val = fl.strip_prefix("vmsize ").unwrap_or("0").trim();
                    vmsize = u64::from_str_radix(val.trim_start_matches("0x"), 16).unwrap_or(0);
                }
            }

            if !name.is_empty() {
                segments.push(ObservedSegment {
                    name,
                    vmaddr,
                    vmsize,
                });
            }
        }

        // Look for LC_MAIN entry point
        if trimmed == "cmd LC_MAIN" {
            while let Some(field_line) = lines.peek() {
                let ft = field_line.trim();
                if ft.starts_with("cmd ") || ft.starts_with("Load command") {
                    break;
                }
                let fl = lines.next().unwrap().trim().to_string();

                if fl.starts_with("entryoff ") {
                    let val = fl.strip_prefix("entryoff ").unwrap_or("0").trim();
                    entry_offset =
                        Some(u64::from_str_radix(val.trim_start_matches("0x"), 16).unwrap_or(0));
                }
            }
        }
    }

    (segments, entry_offset)
}

/// Observed initializer ordering from running with DYLD_PRINT_INITIALIZERS.
#[derive(Debug, Clone)]
pub struct ObservedInitOrder {
    /// Libraries in the order they were initialized.
    pub init_sequence: Vec<String>,
    /// Raw output from DYLD_PRINT_INITIALIZERS.
    pub raw_output: String,
}

/// Collect initializer order by running the binary with DYLD_PRINT_INITIALIZERS=1.
///
/// Returns None if the binary can't be run (e.g., dyld environment stripping for
/// hardened binaries).
pub fn collect_init_order(binary_path: &Path) -> Option<ObservedInitOrder> {
    let output = Command::new(binary_path)
        .env("DYLD_PRINT_INITIALIZERS", "1")
        .output()
        .ok()?;

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let raw = format!("{stderr}{stdout}");

    // DYLD_PRINT_INITIALIZERS output format varies by dyld version.
    // Common patterns:
    //   "dyld[...]: running initializers for /path/to/lib"
    //   "dyld: running initializer ..."
    let mut init_sequence = Vec::new();
    for line in raw.lines() {
        if line.contains("running initializer") || line.contains("running initializers") {
            // Extract the library path/name from the line
            if let Some(path_start) = line.rfind('/') {
                let name = &line[path_start..];
                // Trim trailing whitespace and junk
                let clean = name
                    .trim()
                    .trim_end_matches(|c: char| !c.is_alphanumeric() && c != '.' && c != '_');
                init_sequence.push(clean.to_string());
            } else {
                init_sequence.push(line.trim().to_string());
            }
        }
    }

    Some(ObservedInitOrder {
        init_sequence,
        raw_output: raw,
    })
}

/// Run the full pipeline with symbol binding and initializer comparison.
pub fn run_fixture_full(
    descriptor: &FixtureDescriptor,
) -> Result<(ExecutionPlan, HostObservation, Option<ObservedInitOrder>), Box<dyn std::error::Error>>
{
    let binary_path = &descriptor.binary_path;
    let data = std::fs::read(binary_path)?;
    let parsed = parse(&data)?;

    let plan = ExecutionPlan::from_parsed_full(&parsed, binary_path.clone(), &data);
    let observation = collect_host_observation(binary_path)?;
    let init_order = collect_init_order(binary_path);

    Ok((plan, observation, init_order))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_input() {
        let (segments, entry) = parse_otool_load_commands("");
        assert!(segments.is_empty());
        assert!(entry.is_none());
    }

    #[test]
    fn parse_lc_main() {
        let input = "\
Load command 1
      cmd LC_MAIN
  cmdsize 24
  entryoff 0x3DE0
 stacksize 0
";
        let (segments, entry) = parse_otool_load_commands(input);
        assert!(segments.is_empty());
        assert_eq!(entry, Some(0x3DE0));
    }

    #[test]
    fn parse_lc_segment_64() {
        let input = "\
Load command 0
      cmd LC_SEGMENT_64
  cmdsize 232
  segname __TEXT
   vmaddr 0x0000000100000000
   vmsize 0x0000000000004000
  fileoff 0
 filesize 16384
";
        let (segments, entry) = parse_otool_load_commands(input);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].name, "__TEXT");
        assert_eq!(segments[0].vmaddr, 0x100000000);
        assert_eq!(segments[0].vmsize, 0x4000);
        assert!(entry.is_none());
    }

    #[test]
    fn parse_multiple_segments() {
        let input = "\
Load command 0
      cmd LC_SEGMENT_64
  cmdsize 232
  segname __TEXT
   vmaddr 0x0000000100000000
   vmsize 0x0000000000004000
  fileoff 0
 filesize 16384
Load command 1
      cmd LC_SEGMENT_64
  cmdsize 312
  segname __DATA
   vmaddr 0x0000000100004000
   vmsize 0x0000000000001000
  fileoff 16384
 filesize 4096
";
        let (segments, entry) = parse_otool_load_commands(input);
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].name, "__TEXT");
        assert_eq!(segments[0].vmaddr, 0x100000000);
        assert_eq!(segments[1].name, "__DATA");
        assert_eq!(segments[1].vmaddr, 0x100004000);
        assert_eq!(segments[1].vmsize, 0x1000);
    }

    #[test]
    fn parse_no_entry_point() {
        let input = "\
Load command 0
      cmd LC_SEGMENT_64
  cmdsize 232
  segname __TEXT
   vmaddr 0x0000000100000000
   vmsize 0x0000000000004000
";
        let (segments, entry) = parse_otool_load_commands(input);
        assert_eq!(segments.len(), 1);
        assert!(entry.is_none());
    }

    #[test]
    fn parse_hex_vmaddr() {
        let input = "\
Load command 0
      cmd LC_SEGMENT_64
  cmdsize 72
  segname __DATA_CONST
   vmaddr 0x00000001000a8000
   vmsize 0x0000000000010000
";
        let (segments, _) = parse_otool_load_commands(input);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].vmaddr, 0x1000a8000);
        assert_eq!(segments[0].vmsize, 0x10000);
    }

    #[test]
    fn parse_pagezero() {
        let input = "\
Load command 0
      cmd LC_SEGMENT_64
  cmdsize 72
  segname __PAGEZERO
   vmaddr 0x0000000000000000
   vmsize 0x0000000100000000
";
        let (segments, _) = parse_otool_load_commands(input);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].name, "__PAGEZERO");
        assert_eq!(segments[0].vmaddr, 0);
        assert_eq!(segments[0].vmsize, 0x100000000);
    }

    #[test]
    fn parse_malformed_ignored() {
        let input = "\
this is garbage
random text here
Load command ??
  not a real segment
vmaddr without prefix
";
        let (segments, entry) = parse_otool_load_commands(input);
        assert!(segments.is_empty());
        assert!(entry.is_none());
    }

    #[test]
    fn parse_lc_main_hex_entryoff() {
        let input = "\
Load command 5
      cmd LC_MAIN
  cmdsize 24
  entryoff 0x3DE0
 stacksize 0
";
        let (_, entry) = parse_otool_load_commands(input);
        assert_eq!(entry, Some(0x3DE0));
    }

    #[test]
    fn parse_full_realistic_output() {
        let input = "\
/usr/bin/true:
Load command 0
      cmd LC_SEGMENT_64
  cmdsize 72
  segname __PAGEZERO
   vmaddr 0x0000000000000000
   vmsize 0x0000000100000000
Load command 1
      cmd LC_SEGMENT_64
  cmdsize 232
  segname __TEXT
   vmaddr 0x0000000100000000
   vmsize 0x0000000000004000
  fileoff 0
 filesize 16384
Load command 2
      cmd LC_SEGMENT_64
  cmdsize 312
  segname __DATA
   vmaddr 0x0000000100004000
   vmsize 0x0000000000001000
  fileoff 16384
 filesize 4096
Load command 3
      cmd LC_MAIN
  cmdsize 24
  entryoff 0x3DE0
 stacksize 0
";
        let (segments, entry) = parse_otool_load_commands(input);
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].name, "__PAGEZERO");
        assert_eq!(segments[1].name, "__TEXT");
        assert_eq!(segments[2].name, "__DATA");
        assert_eq!(entry, Some(0x3DE0));
    }
}
