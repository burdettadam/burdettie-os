use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

mod xcodebuild;

pub use xcodebuild::{run_xcodebuild, XCODEBUILD_VERSION};

pub const XCRUN_VERSION: &str = "xcrun version 1 (burdettie)";
pub const XCODE_SELECT_VERSION: &str = "xcode-select version 1 (burdettie)";

const DEFAULT_DEVELOPER_DIR: &str = "/Applications/Xcode.app/Contents/Developer";
const DEFAULT_FLAT_ROOT: &str = "/opt/xcode-tools";
const DEFAULT_IMPORTED_SDK: &str = "/opt/imported-artifacts/sdks/MacOSX.sdk";

#[derive(Clone, Debug, PartialEq, Eq)]
enum ActivePathKind {
    DeveloperDir,
    FlatOverlay,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ActiveDeveloperPath {
    raw: String,
    kind: ActivePathKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum XcrunPlan {
    PrintLine(String),
    Exec {
        program: String,
        args: Vec<String>,
        env: BTreeMap<String, String>,
    },
}

pub fn run_xcrun(args: &[String]) -> i32 {
    let vars = current_env();
    match plan_xcrun(args, &vars) {
        Ok(XcrunPlan::PrintLine(line)) => {
            println!("{line}");
            0
        }
        Ok(XcrunPlan::Exec { program, args, env }) => {
            match Command::new(&program).args(&args).envs(&env).status() {
                Ok(status) => status.code().unwrap_or(1),
                Err(err) => {
                    eprintln!("xcrun: failed to execute '{program}': {err}");
                    1
                }
            }
        }
        Err(message) => {
            eprintln!("{message}");
            1
        }
    }
}

pub fn run_xcode_select(args: &[String]) -> i32 {
    let vars = current_env();
    match args {
        [] => {
            eprintln!("{}", xcode_select_usage());
            1
        }
        [flag] if flag == "--version" => {
            println!("{XCODE_SELECT_VERSION}");
            0
        }
        [flag] if flag == "--help" || flag == "-h" => {
            println!("{}", xcode_select_usage());
            0
        }
        [flag] if flag == "-p" || flag == "--print-path" => {
            match resolve_active_developer_path(&vars) {
                Ok(active) => {
                    println!("{}", active.display_path());
                    0
                }
                Err(message) => {
                    eprintln!("{message}");
                    1
                }
            }
        }
        [flag, path] if flag == "-s" || flag == "--switch" => {
            match switch_developer_path(path, &vars) {
                Ok(()) => 0,
                Err(message) => {
                    eprintln!("{message}");
                    1
                }
            }
        }
        [flag] if flag == "-r" || flag == "--reset" => match reset_developer_path(&vars) {
            Ok(()) => 0,
            Err(message) => {
                eprintln!("{message}");
                1
            }
        },
        [flag] if flag == "--install" => {
            eprintln!("xcode-select: --install is not supported by burdettie yet");
            1
        }
        [flag, ..] => {
            eprintln!(
                "xcode-select: unsupported option '{flag}'\n{}",
                xcode_select_usage()
            );
            1
        }
    }
}

pub fn plan_xcrun(args: &[String], vars: &BTreeMap<String, String>) -> Result<XcrunPlan, String> {
    let active = resolve_active_developer_path(vars)?;
    let mut index = 0usize;
    let mut requested_sdk: Option<String> = None;

    while index < args.len() {
        match args[index].as_str() {
            "--version" => return Ok(XcrunPlan::PrintLine(XCRUN_VERSION.to_string())),
            "--help" | "-h" => return Ok(XcrunPlan::PrintLine(xcrun_usage().to_string())),
            "--show-sdk-path" => return Ok(XcrunPlan::PrintLine(active.sdk_path(vars))),
            "--show-sdk-platform-path" => {
                return Ok(XcrunPlan::PrintLine(active.sdk_platform_path(vars)))
            }
            "--developer-dir" => return Ok(XcrunPlan::PrintLine(active.display_path())),
            "--find" => {
                index += 1;
                let tool = args
                    .get(index)
                    .ok_or_else(|| "xcrun: --find requires a tool name".to_string())?;
                let resolved = active
                    .resolve_tool(tool, vars)
                    .ok_or_else(|| format!("xcrun: unable to find utility '{tool}'"))?;
                return Ok(XcrunPlan::PrintLine(resolved));
            }
            "--sdk" => {
                index += 1;
                let sdk = args
                    .get(index)
                    .ok_or_else(|| "xcrun: --sdk requires a value".to_string())?;
                requested_sdk = Some(sdk.clone());
                if !matches!(sdk.as_str(), "macosx" | "macos") {
                    return Err(format!("xcrun: unsupported sdk '{sdk}'"));
                }
            }
            flag if flag.starts_with('-') => {
                return Err(format!("xcrun: unsupported option '{flag}'"))
            }
            tool => {
                let program = active
                    .resolve_tool(tool, vars)
                    .ok_or_else(|| format!("xcrun: unable to find utility '{tool}'"))?;
                let mut env = BTreeMap::new();
                if requested_sdk.is_some() {
                    env.insert("SDKROOT".to_string(), active.sdk_path(vars));
                }
                return Ok(XcrunPlan::Exec {
                    program,
                    args: args[index + 1..].to_vec(),
                    env,
                });
            }
        }

        index += 1;
    }

    Err("xcrun: missing command or query".to_string())
}

fn current_env() -> BTreeMap<String, String> {
    env::vars().collect()
}

fn xcrun_usage() -> &'static str {
    "usage: xcrun [--find <tool>] [--show-sdk-path] [--show-sdk-platform-path] [tool [args...]]"
}

fn xcode_select_usage() -> &'static str {
    "usage: xcode-select [-p|--print-path] [-s|--switch <path>] [-r|--reset] [--version]"
}

fn resolve_active_developer_path(
    vars: &BTreeMap<String, String>,
) -> Result<ActiveDeveloperPath, String> {
    if let Some(path) = get_non_empty(vars, "DEVELOPER_DIR") {
        return Ok(classify_developer_path(&path));
    }
    if let Some(path) = get_non_empty(vars, "BURDETTIE_XCODE_TOOLS_ROOT") {
        return Ok(ActiveDeveloperPath::flat_overlay(path));
    }
    if let Some(path) = read_selected_developer_path(vars)? {
        return Ok(classify_developer_path(&path));
    }
    if is_flat_overlay_root(DEFAULT_FLAT_ROOT) {
        return Ok(ActiveDeveloperPath::flat_overlay(
            DEFAULT_FLAT_ROOT.to_string(),
        ));
    }
    Ok(ActiveDeveloperPath::developer_dir(
        DEFAULT_DEVELOPER_DIR.to_string(),
    ))
}

fn switch_developer_path(path: &str, vars: &BTreeMap<String, String>) -> Result<(), String> {
    let normalized = normalize_switch_target(path);
    if !path_exists(&normalized) {
        return Err(format!("xcode-select: path does not exist: {normalized}"));
    }
    if !Path::new(&normalized).is_dir() {
        return Err(format!(
            "xcode-select: path is not a directory: {normalized}"
        ));
    }
    let state_file = state_file_path(vars);
    if let Some(parent) = state_file.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "xcode-select: failed to create state dir '{}': {err}",
                parent.display()
            )
        })?;
    }
    fs::write(&state_file, normalized.as_bytes()).map_err(|err| {
        format!(
            "xcode-select: failed to write '{}': {err}",
            state_file.display()
        )
    })
}

fn reset_developer_path(vars: &BTreeMap<String, String>) -> Result<(), String> {
    let state_file = state_file_path(vars);
    match fs::remove_file(&state_file) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!(
            "xcode-select: failed to remove '{}': {err}",
            state_file.display()
        )),
    }
}

fn read_selected_developer_path(vars: &BTreeMap<String, String>) -> Result<Option<String>, String> {
    let state_file = state_file_path(vars);
    match fs::read_to_string(&state_file) {
        Ok(contents) => {
            let trimmed = contents.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                Ok(Some(trimmed.to_string()))
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(format!(
            "xcode-select: failed to read '{}': {err}",
            state_file.display()
        )),
    }
}

fn state_file_path(vars: &BTreeMap<String, String>) -> PathBuf {
    if let Some(path) = get_non_empty(vars, "BURDETTIE_XCODE_SELECT_PATH_FILE") {
        return PathBuf::from(path);
    }
    if let Some(state_home) = get_non_empty(vars, "XDG_STATE_HOME") {
        return PathBuf::from(state_home)
            .join("burdettie")
            .join("xcode-select-path");
    }
    if let Some(home) = get_non_empty(vars, "HOME").or_else(|| get_non_empty(vars, "USERPROFILE")) {
        return PathBuf::from(home)
            .join(".burdettie")
            .join("xcode-select-path");
    }
    env::temp_dir().join("burdettie-xcode-select-path")
}

fn get_non_empty(vars: &BTreeMap<String, String>, key: &str) -> Option<String> {
    vars.get(key)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalize_switch_target(path: &str) -> String {
    if path.ends_with(".app") {
        return format!("{path}/Contents/Developer");
    }
    path.to_string()
}

impl ActiveDeveloperPath {
    fn developer_dir(raw: String) -> Self {
        Self {
            raw,
            kind: ActivePathKind::DeveloperDir,
        }
    }

    fn flat_overlay(raw: String) -> Self {
        Self {
            raw,
            kind: ActivePathKind::FlatOverlay,
        }
    }

    fn display_path(&self) -> String {
        self.raw.clone()
    }

    fn resolve_tool(&self, tool: &str, vars: &BTreeMap<String, String>) -> Option<String> {
        for candidate in self.tool_candidates(tool) {
            if path_exists(&candidate) {
                return Some(candidate);
            }
        }
        if allow_path_fallback(vars) {
            return find_in_path(tool, vars);
        }
        None
    }

    fn tool_candidates(&self, tool: &str) -> Vec<String> {
        match self.kind {
            ActivePathKind::FlatOverlay => vec![
                posix_join(&self.raw, &format!("xcode-tc/{tool}")),
                posix_join(&self.raw, &format!("xcode-dev/{tool}")),
                posix_join(&self.raw, &format!("system/{tool}")),
            ],
            ActivePathKind::DeveloperDir => vec![
                posix_join(
                    &self.raw,
                    &format!("Toolchains/XcodeDefault.xctoolchain/usr/bin/{tool}"),
                ),
                posix_join(&self.raw, &format!("usr/bin/{tool}")),
                posix_join(
                    &self.raw,
                    &format!("Platforms/MacOSX.platform/Developer/usr/bin/{tool}"),
                ),
            ],
        }
    }

    fn sdk_path(&self, vars: &BTreeMap<String, String>) -> String {
        if let Some(sdkroot) = get_non_empty(vars, "SDKROOT") {
            return sdkroot;
        }

        let mut candidates = Vec::new();
        match self.kind {
            ActivePathKind::FlatOverlay => {
                candidates.push(posix_join(&self.raw, "SDKs/MacOSX.sdk"));
                candidates.push(posix_join(
                    &self.raw,
                    "Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk",
                ));
                candidates.push(DEFAULT_IMPORTED_SDK.to_string());
            }
            ActivePathKind::DeveloperDir => {
                candidates.push(posix_join(
                    &self.raw,
                    "Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk",
                ));
            }
        }

        first_existing_or_default(candidates)
    }

    fn sdk_platform_path(&self, vars: &BTreeMap<String, String>) -> String {
        if let Some(sdkroot) = get_non_empty(vars, "SDKROOT") {
            return derive_platform_path(&sdkroot)
                .unwrap_or_else(|| posix_join(&self.raw, "Platforms/MacOSX.platform"));
        }
        match self.kind {
            ActivePathKind::FlatOverlay => posix_join(&self.raw, "Platforms/MacOSX.platform"),
            ActivePathKind::DeveloperDir => posix_join(&self.raw, "Platforms/MacOSX.platform"),
        }
    }
}

fn classify_developer_path(path: &str) -> ActiveDeveloperPath {
    if is_flat_overlay_root(path) {
        return ActiveDeveloperPath::flat_overlay(path.to_string());
    }
    if path.ends_with(".app") {
        return ActiveDeveloperPath::developer_dir(format!("{path}/Contents/Developer"));
    }
    ActiveDeveloperPath::developer_dir(path.to_string())
}

fn is_flat_overlay_root(path: &str) -> bool {
    path_exists(&posix_join(path, "xcode-tc")) || path_exists(&posix_join(path, "xcode-dev"))
}

fn path_exists(path: &str) -> bool {
    Path::new(path).exists()
}

fn first_existing_or_default(candidates: Vec<String>) -> String {
    for candidate in &candidates {
        if path_exists(candidate) {
            return candidate.clone();
        }
    }
    candidates
        .into_iter()
        .next()
        .unwrap_or_else(|| DEFAULT_IMPORTED_SDK.to_string())
}

fn derive_platform_path(sdkroot: &str) -> Option<String> {
    let marker = "/Developer/SDKs/";
    sdkroot.find(marker).and_then(|index| {
        let prefix = &sdkroot[..index];
        if prefix.ends_with(".platform") {
            Some(prefix.to_string())
        } else {
            None
        }
    })
}

fn posix_join(base: &str, suffix: &str) -> String {
    if base.ends_with('/') {
        format!("{base}{suffix}")
    } else {
        format!("{base}/{suffix}")
    }
}

fn find_in_path(tool: &str, vars: &BTreeMap<String, String>) -> Option<String> {
    let path_value = vars.get("PATH")?;
    for dir in env::split_paths(path_value) {
        for name in executable_names(tool, vars) {
            let candidate = dir.join(&name);
            if candidate.is_file() {
                return Some(normalize_output_path(&candidate));
            }
        }
    }
    None
}

fn allow_path_fallback(vars: &BTreeMap<String, String>) -> bool {
    get_non_empty(vars, "BURDETTIE_XCRUN_ALLOW_PATH_FALLBACK")
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn executable_names(tool: &str, vars: &BTreeMap<String, String>) -> Vec<String> {
    #[cfg(windows)]
    {
        if Path::new(tool).extension().is_some() {
            return vec![tool.to_string()];
        }
        let path_ext = vars
            .get("PATHEXT")
            .cloned()
            .unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".to_string());
        let mut names = vec![tool.to_string()];
        for ext in path_ext.split(';').filter(|ext| !ext.is_empty()) {
            names.push(format!("{tool}{ext}"));
        }
        names
    }
    #[cfg(not(windows))]
    {
        let _ = vars;
        vec![tool.to_string()]
    }
}

fn normalize_output_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_env(temp: &TempDir) -> BTreeMap<String, String> {
        let mut vars = BTreeMap::new();
        vars.insert(
            "BURDETTIE_XCODE_SELECT_PATH_FILE".to_string(),
            normalize_output_path(&temp.path().join("xcode-select-path")),
        );
        vars
    }

    #[test]
    fn switch_and_reset_round_trip() {
        let temp = TempDir::new().expect("tempdir");
        let vars = test_env(&temp);
        let selected_app = temp.path().join("Selected.app");
        fs::create_dir_all(selected_app.join("Contents/Developer"))
            .expect("create selected app developer dir");
        let selected = normalize_output_path(&selected_app);

        switch_developer_path(&selected, &vars).expect("switch succeeds");

        let active = resolve_active_developer_path(&vars).expect("active path resolves");
        assert_eq!(
            active.display_path(),
            format!("{selected}/Contents/Developer")
        );

        reset_developer_path(&vars).expect("reset succeeds");
        let active = resolve_active_developer_path(&vars).expect("default path resolves");
        assert_eq!(active.display_path(), DEFAULT_DEVELOPER_DIR);
    }

    #[test]
    fn developer_dir_env_wins() {
        let temp = TempDir::new().expect("tempdir");
        let mut vars = test_env(&temp);
        vars.insert("DEVELOPER_DIR".to_string(), "/custom/Developer".to_string());

        let active = resolve_active_developer_path(&vars).expect("active path resolves");
        assert_eq!(active.display_path(), "/custom/Developer");
    }

    #[test]
    fn flat_overlay_prefers_toolchain_binary() {
        let temp = TempDir::new().expect("tempdir");
        let tc_dir = temp.path().join("xcode-tc");
        let dev_dir = temp.path().join("xcode-dev");
        fs::create_dir_all(&tc_dir).expect("create toolchain dir");
        fs::create_dir_all(&dev_dir).expect("create dev dir");
        fs::write(tc_dir.join("clang"), b"").expect("write toolchain clang");
        fs::write(dev_dir.join("clang"), b"").expect("write dev clang");

        let mut vars = test_env(&temp);
        vars.insert(
            "BURDETTIE_XCODE_TOOLS_ROOT".to_string(),
            normalize_output_path(temp.path()),
        );

        let plan =
            plan_xcrun(&["--find".to_string(), "clang".to_string()], &vars).expect("plan resolves");

        assert_eq!(
            plan,
            XcrunPlan::PrintLine(normalize_output_path(&tc_dir.join("clang")))
        );
    }

    #[test]
    fn flat_overlay_requires_known_subdirs() {
        let temp = TempDir::new().expect("tempdir");
        let root = normalize_output_path(temp.path());

        assert!(!is_flat_overlay_root(&root));

        fs::create_dir_all(temp.path().join("xcode-dev")).expect("create flat overlay marker");

        assert!(is_flat_overlay_root(&root));
    }

    #[test]
    fn sdkroot_overrides_default_sdk_path() {
        let temp = TempDir::new().expect("tempdir");
        let mut vars = test_env(&temp);
        vars.insert(
            "SDKROOT".to_string(),
            "/custom/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk".to_string(),
        );

        let plan = plan_xcrun(&["--show-sdk-path".to_string()], &vars).expect("plan resolves");
        assert_eq!(
            plan,
            XcrunPlan::PrintLine(
                "/custom/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk".to_string(),
            )
        );

        let platform_plan =
            plan_xcrun(&["--show-sdk-platform-path".to_string()], &vars).expect("plan resolves");
        assert_eq!(
            platform_plan,
            XcrunPlan::PrintLine("/custom/Platforms/MacOSX.platform".to_string())
        );
    }

    #[test]
    fn xcrun_exec_plan_uses_path_fallback() {
        let temp = TempDir::new().expect("tempdir");
        let bin_dir = temp.path().join("bin");
        fs::create_dir_all(&bin_dir).expect("create bin dir");

        #[cfg(windows)]
        let tool_name = "demo-tool.exe";
        #[cfg(not(windows))]
        let tool_name = "demo-tool";

        fs::write(bin_dir.join(tool_name), b"").expect("write tool file");

        let mut vars = test_env(&temp);
        vars.insert("PATH".to_string(), normalize_output_path(&bin_dir));
        vars.insert(
            "BURDETTIE_XCRUN_ALLOW_PATH_FALLBACK".to_string(),
            "1".to_string(),
        );

        let plan = plan_xcrun(&["demo-tool".to_string(), "--flag".to_string()], &vars)
            .expect("plan resolves");

        assert_eq!(
            plan,
            XcrunPlan::Exec {
                program: normalize_output_path(&bin_dir.join(tool_name)),
                args: vec!["--flag".to_string()],
                env: BTreeMap::new(),
            }
        );
    }

    #[test]
    fn xcrun_path_fallback_is_disabled_by_default() {
        let temp = TempDir::new().expect("tempdir");
        let bin_dir = temp.path().join("bin");
        fs::create_dir_all(&bin_dir).expect("create bin dir");

        #[cfg(windows)]
        let tool_name = "demo-tool.exe";
        #[cfg(not(windows))]
        let tool_name = "demo-tool";

        fs::write(bin_dir.join(tool_name), b"").expect("write tool file");

        let mut vars = test_env(&temp);
        vars.insert("PATH".to_string(), normalize_output_path(&bin_dir));

        let result = plan_xcrun(&["--find".to_string(), "demo-tool".to_string()], &vars);
        assert!(
            result.is_err(),
            "PATH fallback should require explicit opt-in"
        );
    }

    #[test]
    fn xcrun_exec_plan_sets_sdkroot_when_sdk_is_requested() {
        let temp = TempDir::new().expect("tempdir");
        let tc_dir = temp.path().join("xcode-tc");
        fs::create_dir_all(&tc_dir).expect("create toolchain dir");
        fs::write(tc_dir.join("clang"), b"").expect("write clang");

        let mut vars = test_env(&temp);
        vars.insert(
            "BURDETTIE_XCODE_TOOLS_ROOT".to_string(),
            normalize_output_path(temp.path()),
        );
        vars.insert(
            "SDKROOT".to_string(),
            "/custom/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk".to_string(),
        );

        let plan = plan_xcrun(
            &[
                "--sdk".to_string(),
                "macosx".to_string(),
                "clang".to_string(),
                "--version".to_string(),
            ],
            &vars,
        )
        .expect("plan resolves");

        assert_eq!(
            plan,
            XcrunPlan::Exec {
                program: normalize_output_path(&tc_dir.join("clang")),
                args: vec!["--version".to_string()],
                env: BTreeMap::from([(
                    "SDKROOT".to_string(),
                    "/custom/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk".to_string(),
                )]),
            }
        );
    }

    #[test]
    fn switch_rejects_missing_path() {
        let temp = TempDir::new().expect("tempdir");
        let vars = test_env(&temp);
        let missing = normalize_output_path(&temp.path().join("Missing.app"));

        let error = switch_developer_path(&missing, &vars).expect_err("missing path should fail");
        assert!(error.contains("path does not exist"));
    }

    #[test]
    fn xcode_select_print_path_uses_flat_overlay_root() {
        let temp = TempDir::new().expect("tempdir");
        fs::create_dir_all(temp.path().join("xcode-dev")).expect("create xcode-dev");

        let mut vars = test_env(&temp);
        vars.insert(
            "BURDETTIE_XCODE_TOOLS_ROOT".to_string(),
            normalize_output_path(temp.path()),
        );

        let active = resolve_active_developer_path(&vars).expect("active path resolves");
        assert_eq!(active.display_path(), normalize_output_path(temp.path()));
    }
}
