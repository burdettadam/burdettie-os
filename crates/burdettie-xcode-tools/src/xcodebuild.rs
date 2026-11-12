use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{
    find_in_path, get_non_empty, normalize_output_path, posix_join, resolve_active_developer_path,
};

pub const XCODEBUILD_VERSION: &str = "xcodebuild version 1 (burdettie)";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum XcodebuildPlan {
    Print(String),
    Build(BuildPlan),
    Clean { build_dir: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildPlan {
    pub compiler: String,
    pub args: Vec<String>,
    pub current_dir: String,
    pub output: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    List,
    ShowBuildSettings,
    Build,
    Clean,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CliOptions {
    action: Action,
    project: PathBuf,
    target: Option<String>,
    configuration: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProjectModel {
    project_name: String,
    project_dir: PathBuf,
    project_bundle: PathBuf,
    default_configuration: String,
    build_configurations: Vec<String>,
    project_build_settings: BTreeMap<String, BTreeMap<String, String>>,
    targets: Vec<TargetModel>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TargetModel {
    name: String,
    product_name: String,
    product_type: String,
    default_configuration: String,
    build_settings: BTreeMap<String, BTreeMap<String, String>>,
    sources: Vec<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ConfigList {
    default_name: Option<String>,
    settings_by_name: BTreeMap<String, BTreeMap<String, String>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum OpenStepValue {
    String(String),
    Array(Vec<OpenStepValue>),
    Dict(BTreeMap<String, OpenStepValue>),
}

pub fn run_xcodebuild(args: &[String]) -> i32 {
    let vars = super::current_env();
    let cwd = match env::current_dir() {
        Ok(path) => path,
        Err(err) => {
            eprintln!("xcodebuild: failed to read current dir: {err}");
            return 1;
        }
    };

    match plan_xcodebuild(args, &vars, &cwd) {
        Ok(XcodebuildPlan::Print(output)) => {
            print!("{output}");
            0
        }
        Ok(XcodebuildPlan::Build(plan)) => run_build_plan(&plan),
        Ok(XcodebuildPlan::Clean { build_dir }) => {
            let path = PathBuf::from(&build_dir);
            match fs::remove_dir_all(&path) {
                Ok(()) => {
                    println!("** CLEAN SUCCEEDED **");
                    0
                }
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                    println!("** CLEAN SUCCEEDED **");
                    0
                }
                Err(err) => {
                    eprintln!("xcodebuild: clean failed for '{}': {err}", path.display());
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

fn plan_xcodebuild(
    args: &[String],
    vars: &BTreeMap<String, String>,
    cwd: &Path,
) -> Result<XcodebuildPlan, String> {
    if matches!(args, [flag] if flag == "--version" || flag == "-version") {
        return Ok(XcodebuildPlan::Print(format!("{XCODEBUILD_VERSION}\n")));
    }
    if matches!(args, [flag] if flag == "--help" || flag == "-help" || flag == "-h") {
        return Ok(XcodebuildPlan::Print(format!("{}\n", xcodebuild_usage())));
    }

    let options = parse_cli(args, cwd)?;
    let model = load_project(&options.project)?;
    let target = select_target(&model, options.target.as_deref())?;
    let configuration = select_configuration(&model, target, options.configuration.as_deref())?;

    match options.action {
        Action::List => Ok(XcodebuildPlan::Print(format_list(&model))),
        Action::ShowBuildSettings => Ok(XcodebuildPlan::Print(format_build_settings(
            target,
            &resolve_build_settings(&model, target, &configuration, vars)?,
        ))),
        Action::Build => Ok(XcodebuildPlan::Build(create_build_plan(
            &model,
            target,
            &resolve_build_settings(&model, target, &configuration, vars)?,
            vars,
        )?)),
        Action::Clean => {
            let settings = resolve_build_settings(&model, target, &configuration, vars)?;
            let build_dir = settings.get("BUILD_DIR").cloned().ok_or_else(|| {
                "xcodebuild: BUILD_DIR missing from resolved settings".to_string()
            })?;
            Ok(XcodebuildPlan::Clean { build_dir })
        }
    }
}

fn run_build_plan(plan: &BuildPlan) -> i32 {
    let output_path = PathBuf::from(&plan.output);
    if let Some(parent) = output_path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            eprintln!("xcodebuild: failed to create '{}': {err}", parent.display());
            return 1;
        }
    }

    match Command::new(&plan.compiler)
        .current_dir(Path::new(&plan.current_dir))
        .args(&plan.args)
        .status()
    {
        Ok(status) if status.success() => {
            println!("** BUILD SUCCEEDED **");
            println!("{}", plan.output);
            0
        }
        Ok(status) => {
            eprintln!(
                "xcodebuild: build failed with exit code {}",
                status.code().unwrap_or(1)
            );
            1
        }
        Err(err) => {
            eprintln!("xcodebuild: failed to execute '{}': {err}", plan.compiler);
            1
        }
    }
}

fn xcodebuild_usage() -> &'static str {
    "usage: xcodebuild [-project <path>] [-target <name>] [-configuration <name>] [-list|-showBuildSettings|build|clean]"
}

fn parse_cli(args: &[String], cwd: &Path) -> Result<CliOptions, String> {
    let mut action = Action::Build;
    let mut project: Option<PathBuf> = None;
    let mut target: Option<String> = None;
    let mut configuration: Option<String> = None;
    let mut index = 0usize;

    while index < args.len() {
        match args[index].as_str() {
            "-project" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "xcodebuild: -project requires a value".to_string())?;
                project = Some(PathBuf::from(value));
            }
            "-target" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "xcodebuild: -target requires a value".to_string())?;
                target = Some(value.clone());
            }
            "-configuration" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "xcodebuild: -configuration requires a value".to_string())?;
                configuration = Some(value.clone());
            }
            "-list" => action = Action::List,
            "-showBuildSettings" => action = Action::ShowBuildSettings,
            "build" => action = Action::Build,
            "clean" => action = Action::Clean,
            flag if flag.starts_with('-') => {
                return Err(format!("xcodebuild: unsupported option '{flag}'"));
            }
            action_name => {
                return Err(format!("xcodebuild: unsupported action '{action_name}'"));
            }
        }

        index += 1;
    }

    let project = match project {
        Some(path) => normalize_project_path(cwd, &path),
        None => discover_project(cwd)?,
    };

    Ok(CliOptions {
        action,
        project,
        target,
        configuration,
    })
}

fn normalize_project_path(cwd: &Path, value: &Path) -> PathBuf {
    if value.is_absolute() {
        value.to_path_buf()
    } else {
        cwd.join(value)
    }
}

fn discover_project(cwd: &Path) -> Result<PathBuf, String> {
    let mut candidates = Vec::new();
    let entries = fs::read_dir(cwd)
        .map_err(|err| format!("xcodebuild: failed to read '{}': {err}", cwd.display()))?;

    for entry in entries {
        let entry = entry
            .map_err(|err| format!("xcodebuild: failed to scan '{}': {err}", cwd.display()))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("xcodeproj") {
            candidates.push(path);
        }
    }

    match candidates.len() {
        1 => Ok(candidates.remove(0)),
        0 => Err("xcodebuild: no .xcodeproj found; pass -project <path>".to_string()),
        _ => Err("xcodebuild: multiple .xcodeproj entries found; pass -project <path>".to_string()),
    }
}

fn load_project(project_bundle: &Path) -> Result<ProjectModel, String> {
    let project_file = project_bundle.join("project.pbxproj");
    let contents = fs::read_to_string(&project_file).map_err(|err| {
        format!(
            "xcodebuild: failed to read '{}': {err}",
            project_file.display()
        )
    })?;
    let value = OpenStepParser::new(&contents).parse()?;
    let root = dict_value(&value)?;
    let root_object_id = string_field(root, "rootObject")?;
    let objects = dict_field(root, "objects")?;
    let root_object = object_dict(objects, root_object_id)?;

    if string_field(root_object, "isa")? != "PBXProject" {
        return Err("xcodebuild: rootObject is not a PBXProject".to_string());
    }

    let project_config_list = load_config_list(
        objects,
        string_field(root_object, "buildConfigurationList")?,
    )?;
    let target_ids = string_array(field(root_object, "targets")?)?;
    let project_dir = project_bundle
        .parent()
        .ok_or_else(|| {
            format!(
                "xcodebuild: '{}' has no parent directory",
                project_bundle.display()
            )
        })?
        .to_path_buf();
    let project_name = project_bundle
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            format!(
                "xcodebuild: invalid project bundle name '{}'",
                project_bundle.display()
            )
        })?
        .to_string();

    let mut targets = Vec::new();
    for target_id in target_ids {
        let target_dict = object_dict(objects, &target_id)?;
        if string_field(target_dict, "isa")? != "PBXNativeTarget" {
            continue;
        }

        let target_name = string_field(target_dict, "name")?.to_string();
        let product_name = optional_string_field(target_dict, "productName")
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| target_name.clone());
        let product_type = string_field(target_dict, "productType")?.to_string();
        let config_list = load_config_list(
            objects,
            string_field(target_dict, "buildConfigurationList")?,
        )?;
        let mut sources = Vec::new();

        for build_phase_id in string_array(field(target_dict, "buildPhases")?)? {
            let phase_dict = object_dict(objects, &build_phase_id)?;
            if string_field(phase_dict, "isa")? != "PBXSourcesBuildPhase" {
                continue;
            }
            for build_file_id in string_array(field(phase_dict, "files")?)? {
                let build_file = object_dict(objects, &build_file_id)?;
                if string_field(build_file, "isa")? != "PBXBuildFile" {
                    continue;
                }
                let file_ref = string_field(build_file, "fileRef")?;
                let file_dict = object_dict(objects, file_ref)?;
                let relative = string_field(file_dict, "path")?;
                let source_tree =
                    optional_string_field(file_dict, "sourceTree").unwrap_or("<group>");
                let resolved = resolve_source_path(&project_dir, source_tree, relative)?;
                sources.push(resolved);
            }
        }

        targets.push(TargetModel {
            name: target_name,
            product_name,
            product_type,
            default_configuration: config_list.default_name.clone().unwrap_or_else(|| {
                project_config_list
                    .default_name
                    .clone()
                    .unwrap_or_else(|| "Debug".to_string())
            }),
            build_settings: config_list.settings_by_name,
            sources,
        });
    }

    Ok(ProjectModel {
        project_name,
        project_dir,
        project_bundle: project_bundle.to_path_buf(),
        default_configuration: project_config_list
            .default_name
            .clone()
            .unwrap_or_else(|| "Debug".to_string()),
        build_configurations: project_config_list
            .settings_by_name
            .keys()
            .cloned()
            .collect(),
        project_build_settings: project_config_list.settings_by_name,
        targets,
    })
}

fn resolve_source_path(
    project_dir: &Path,
    source_tree: &str,
    relative: &str,
) -> Result<PathBuf, String> {
    match source_tree {
        "SOURCE_ROOT" | "<group>" => Ok(project_dir.join(relative)),
        other => Err(format!("xcodebuild: unsupported sourceTree '{other}'")),
    }
}

fn load_config_list(
    objects: &BTreeMap<String, OpenStepValue>,
    object_id: &str,
) -> Result<ConfigList, String> {
    let config_list = object_dict(objects, object_id)?;
    if string_field(config_list, "isa")? != "XCConfigurationList" {
        return Err(format!(
            "xcodebuild: object '{object_id}' is not an XCConfigurationList"
        ));
    }

    let mut settings_by_name = BTreeMap::new();
    for config_id in string_array(field(config_list, "buildConfigurations")?)? {
        let config = object_dict(objects, &config_id)?;
        if string_field(config, "isa")? != "XCBuildConfiguration" {
            continue;
        }
        let name = string_field(config, "name")?.to_string();
        let settings = match config.get("buildSettings") {
            Some(value) => string_map(value)?,
            None => BTreeMap::new(),
        };
        settings_by_name.insert(name, settings);
    }

    Ok(ConfigList {
        default_name: optional_string_field(config_list, "defaultConfigurationName")
            .map(ToOwned::to_owned),
        settings_by_name,
    })
}

fn select_target<'a>(
    model: &'a ProjectModel,
    requested: Option<&str>,
) -> Result<&'a TargetModel, String> {
    match requested {
        Some(name) => model
            .targets
            .iter()
            .find(|target| target.name == name)
            .ok_or_else(|| format!("xcodebuild: unknown target '{name}'")),
        None => model
            .targets
            .first()
            .ok_or_else(|| "xcodebuild: project has no native targets".to_string()),
    }
}

fn select_configuration(
    model: &ProjectModel,
    target: &TargetModel,
    requested: Option<&str>,
) -> Result<String, String> {
    let configuration = requested
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| target.default_configuration.clone());

    if target.build_settings.contains_key(&configuration)
        || model.project_build_settings.contains_key(&configuration)
    {
        Ok(configuration)
    } else {
        Err(format!(
            "xcodebuild: unknown build configuration '{configuration}'"
        ))
    }
}

fn resolve_build_settings(
    model: &ProjectModel,
    target: &TargetModel,
    configuration: &str,
    vars: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, String> {
    let active = resolve_active_developer_path(vars)?;
    let project_dir = normalize_output_path(&model.project_dir);
    let project_bundle = normalize_output_path(&model.project_bundle);

    let mut settings = BTreeMap::new();
    settings.insert("DEVELOPER_DIR".to_string(), active.display_path());
    settings.insert("SDKROOT".to_string(), sdkroot_for_xcodebuild(&active, vars));
    settings.insert("PROJECT_DIR".to_string(), project_dir.clone());
    settings.insert("SRCROOT".to_string(), project_dir.clone());
    settings.insert("PROJECT_FILE_PATH".to_string(), project_bundle);
    settings.insert("PROJECT_NAME".to_string(), model.project_name.clone());
    settings.insert("TARGET_NAME".to_string(), target.name.clone());
    settings.insert("CONFIGURATION".to_string(), configuration.to_string());
    settings.insert("PRODUCT_NAME".to_string(), target.product_name.clone());
    settings.insert("BUILD_DIR".to_string(), posix_join(&project_dir, "build"));
    settings.insert("SYMROOT".to_string(), "$(BUILD_DIR)".to_string());
    settings.insert(
        "OBJROOT".to_string(),
        posix_join(&posix_join(&project_dir, "build"), "Intermediates.noindex"),
    );
    settings.insert(
        "CONFIGURATION_BUILD_DIR".to_string(),
        "$(BUILD_DIR)/$(CONFIGURATION)".to_string(),
    );
    settings.insert(
        "TARGET_BUILD_DIR".to_string(),
        "$(CONFIGURATION_BUILD_DIR)".to_string(),
    );
    settings.insert("EXECUTABLE_PATH".to_string(), "$(PRODUCT_NAME)".to_string());
    settings.insert(
        "FULL_PRODUCT_NAME".to_string(),
        "$(PRODUCT_NAME)".to_string(),
    );

    if let Some(project_settings) = model.project_build_settings.get(configuration) {
        settings.extend(project_settings.clone());
    }
    if let Some(target_settings) = target.build_settings.get(configuration) {
        settings.extend(target_settings.clone());
    }

    settings
        .entry("PRODUCT_NAME".to_string())
        .or_insert_with(|| target.product_name.clone());
    settings
        .entry("BUILD_DIR".to_string())
        .or_insert_with(|| posix_join(&project_dir, "build"));
    settings
        .entry("SYMROOT".to_string())
        .or_insert_with(|| "$(BUILD_DIR)".to_string());
    settings
        .entry("OBJROOT".to_string())
        .or_insert_with(|| "$(BUILD_DIR)/Intermediates.noindex".to_string());
    settings
        .entry("CONFIGURATION_BUILD_DIR".to_string())
        .or_insert_with(|| "$(BUILD_DIR)/$(CONFIGURATION)".to_string());
    settings
        .entry("TARGET_BUILD_DIR".to_string())
        .or_insert_with(|| "$(CONFIGURATION_BUILD_DIR)".to_string());
    settings
        .entry("EXECUTABLE_PATH".to_string())
        .or_insert_with(|| "$(PRODUCT_NAME)".to_string());
    settings
        .entry("FULL_PRODUCT_NAME".to_string())
        .or_insert_with(|| "$(PRODUCT_NAME)".to_string());

    expand_settings(&mut settings);
    Ok(settings)
}

fn expand_settings(settings: &mut BTreeMap<String, String>) {
    for _ in 0..8 {
        let snapshot = settings.clone();
        let mut changed = false;
        for value in settings.values_mut() {
            let expanded = expand_value(value, &snapshot);
            if expanded != *value {
                *value = expanded;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

fn expand_value(value: &str, settings: &BTreeMap<String, String>) -> String {
    let mut result = String::new();
    let bytes = value.as_bytes();
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] == b'$' && index + 2 < bytes.len() {
            let (open, close) = match bytes[index + 1] {
                b'(' => (b'(', b')'),
                b'{' => (b'{', b'}'),
                _ => {
                    result.push(bytes[index] as char);
                    index += 1;
                    continue;
                }
            };

            if let Some(end) = bytes[index + 2..].iter().position(|byte| *byte == close) {
                let key_start = index + 2;
                let key_end = key_start + end;
                let key = &value[key_start..key_end];
                if let Some(replacement) = settings.get(key) {
                    result.push_str(replacement);
                } else {
                    result.push('$');
                    result.push(open as char);
                    result.push_str(key);
                    result.push(close as char);
                }
                index = key_end + 1;
                continue;
            }
        }

        result.push(bytes[index] as char);
        index += 1;
    }

    result
}

fn format_list(model: &ProjectModel) -> String {
    let mut output = format!("Information about project \"{}\":\n", model.project_name);
    output.push_str("    Targets:\n");
    for target in &model.targets {
        output.push_str(&format!("        {}\n", target.name));
    }
    output.push_str("\n    Build Configurations:\n");
    for configuration in &model.build_configurations {
        output.push_str(&format!("        {}\n", configuration));
    }
    output.push_str(&format!(
        "\nIf no build configuration is specified and -scheme is not passed then \"{}\" is used.\n",
        model.default_configuration
    ));
    output
}

fn format_build_settings(target: &TargetModel, settings: &BTreeMap<String, String>) -> String {
    let mut output = format!(
        "Build settings for action build and target {}:\n",
        target.name
    );
    for (key, value) in settings {
        output.push_str(&format!("    {key} = {value}\n"));
    }
    output
}

fn create_build_plan(
    model: &ProjectModel,
    target: &TargetModel,
    settings: &BTreeMap<String, String>,
    vars: &BTreeMap<String, String>,
) -> Result<BuildPlan, String> {
    if target.product_type != "com.apple.product-type.tool" {
        return Err(format!(
            "xcodebuild: unsupported productType '{}' for target '{}'",
            target.product_type, target.name
        ));
    }
    if target.sources.is_empty() {
        return Err(format!(
            "xcodebuild: target '{}' has no source files",
            target.name
        ));
    }

    let compiler = compiler_for_build(vars)?;
    let output_dir = settings
        .get("TARGET_BUILD_DIR")
        .ok_or_else(|| "xcodebuild: TARGET_BUILD_DIR missing from resolved settings".to_string())?;
    let executable = settings
        .get("EXECUTABLE_PATH")
        .ok_or_else(|| "xcodebuild: EXECUTABLE_PATH missing from resolved settings".to_string())?;
    let output = normalize_output_path(&PathBuf::from(output_dir).join(executable));
    let mut args = Vec::new();

    for source in &target.sources {
        args.push(normalize_output_path(source));
    }
    for include_dir in split_setting(settings.get("HEADER_SEARCH_PATHS").map(String::as_str)) {
        args.push(format!("-I{include_dir}"));
    }
    args.extend(split_setting(
        settings.get("OTHER_CFLAGS").map(String::as_str),
    ));
    args.extend(split_setting(
        settings.get("OTHER_LDFLAGS").map(String::as_str),
    ));
    args.push("-o".to_string());
    args.push(output.clone());

    Ok(BuildPlan {
        compiler,
        args,
        current_dir: normalize_output_path(&model.project_dir),
        output,
    })
}

fn compiler_for_build(vars: &BTreeMap<String, String>) -> Result<String, String> {
    if let Some(value) =
        get_non_empty(vars, "BURDETTIE_XCODEBUILD_CC").or_else(|| get_non_empty(vars, "CC"))
    {
        return Ok(value);
    }

    for tool in ["cc", "clang"] {
        if let Some(path) = find_in_path(tool, vars) {
            if !is_overlay_tool_path(&path, vars) {
                return Ok(path);
            }
        }
    }

    Err("xcodebuild: no compiler found; set CC or BURDETTIE_XCODEBUILD_CC".to_string())
}

fn is_overlay_tool_path(path: &str, vars: &BTreeMap<String, String>) -> bool {
    let candidate = normalize_prefix(path);
    let mut roots = vec![normalize_prefix(super::DEFAULT_FLAT_ROOT)];
    if let Some(root) = get_non_empty(vars, "BURDETTIE_XCODE_TOOLS_ROOT") {
        roots.push(normalize_prefix(&root));
    }

    roots
        .into_iter()
        .any(|root| candidate == root || candidate.starts_with(&(root + "/")))
}

fn normalize_prefix(path: &str) -> String {
    path.replace('\\', "/").trim_end_matches('/').to_string()
}

fn sdkroot_for_xcodebuild(
    active: &super::ActiveDeveloperPath,
    vars: &BTreeMap<String, String>,
) -> String {
    if let Some(value) = get_non_empty(vars, "SDKROOT") {
        return value;
    }

    let resolved = active.sdk_path(vars);
    if Path::new(&resolved).exists() {
        resolved
    } else {
        "macosx".to_string()
    }
}

fn split_setting(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect()
}

fn object_dict<'a>(
    objects: &'a BTreeMap<String, OpenStepValue>,
    object_id: &str,
) -> Result<&'a BTreeMap<String, OpenStepValue>, String> {
    let value = objects
        .get(object_id)
        .ok_or_else(|| format!("xcodebuild: object '{object_id}' missing from project"))?;
    dict_value(value)
}

fn field<'a>(
    dict: &'a BTreeMap<String, OpenStepValue>,
    key: &str,
) -> Result<&'a OpenStepValue, String> {
    dict.get(key)
        .ok_or_else(|| format!("xcodebuild: missing field '{key}'"))
}

fn dict_field<'a>(
    dict: &'a BTreeMap<String, OpenStepValue>,
    key: &str,
) -> Result<&'a BTreeMap<String, OpenStepValue>, String> {
    dict_value(field(dict, key)?)
}

fn dict_value(value: &OpenStepValue) -> Result<&BTreeMap<String, OpenStepValue>, String> {
    match value {
        OpenStepValue::Dict(dict) => Ok(dict),
        _ => Err("xcodebuild: expected dictionary value".to_string()),
    }
}

fn string_field<'a>(
    dict: &'a BTreeMap<String, OpenStepValue>,
    key: &str,
) -> Result<&'a str, String> {
    string_value(field(dict, key)?)
}

fn optional_string_field<'a>(
    dict: &'a BTreeMap<String, OpenStepValue>,
    key: &str,
) -> Option<&'a str> {
    dict.get(key).and_then(|value| string_value(value).ok())
}

fn string_value(value: &OpenStepValue) -> Result<&str, String> {
    match value {
        OpenStepValue::String(value) => Ok(value),
        _ => Err("xcodebuild: expected string value".to_string()),
    }
}

fn string_array(value: &OpenStepValue) -> Result<Vec<String>, String> {
    match value {
        OpenStepValue::Array(items) => items
            .iter()
            .map(|item| string_value(item).map(ToOwned::to_owned))
            .collect(),
        _ => Err("xcodebuild: expected array value".to_string()),
    }
}

fn string_map(value: &OpenStepValue) -> Result<BTreeMap<String, String>, String> {
    let dict = dict_value(value)?;
    let mut result = BTreeMap::new();

    for (key, value) in dict {
        let mapped = match value {
            OpenStepValue::String(value) => value.clone(),
            OpenStepValue::Array(items) => items
                .iter()
                .map(string_value)
                .collect::<Result<Vec<_>, _>>()?
                .join(" "),
            OpenStepValue::Dict(_) => {
                return Err(format!(
                    "xcodebuild: nested dictionary is not supported for build setting '{key}'"
                ));
            }
        };
        result.insert(key.clone(), mapped);
    }

    Ok(result)
}

struct OpenStepParser<'a> {
    input: &'a [u8],
    index: usize,
}

impl<'a> OpenStepParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            index: 0,
        }
    }

    fn parse(mut self) -> Result<OpenStepValue, String> {
        let value = self.parse_value()?;
        self.skip_ignored()?;
        if self.index != self.input.len() {
            return Err("xcodebuild: unexpected trailing input in project.pbxproj".to_string());
        }
        Ok(value)
    }

    fn parse_value(&mut self) -> Result<OpenStepValue, String> {
        self.skip_ignored()?;
        match self.peek() {
            Some(b'{') => self.parse_dict(),
            Some(b'(') => self.parse_array(),
            Some(b'"') => self.parse_quoted_string().map(OpenStepValue::String),
            Some(_) => self.parse_unquoted_string().map(OpenStepValue::String),
            None => Err("xcodebuild: unexpected end of input while parsing value".to_string()),
        }
    }

    fn parse_dict(&mut self) -> Result<OpenStepValue, String> {
        self.expect(b'{')?;
        let mut dict = BTreeMap::new();
        loop {
            self.skip_ignored()?;
            if self.try_consume(b'}') {
                break;
            }
            let key = self.parse_string()?;
            self.skip_ignored()?;
            self.expect(b'=')?;
            let value = self.parse_value()?;
            self.skip_ignored()?;
            self.expect(b';')?;
            dict.insert(key, value);
        }
        Ok(OpenStepValue::Dict(dict))
    }

    fn parse_array(&mut self) -> Result<OpenStepValue, String> {
        self.expect(b'(')?;
        let mut items = Vec::new();
        loop {
            self.skip_ignored()?;
            if self.try_consume(b')') {
                break;
            }
            let value = self.parse_value()?;
            items.push(value);
            self.skip_ignored()?;
            if self.try_consume(b',') {
                continue;
            }
            self.expect(b')')?;
            break;
        }
        Ok(OpenStepValue::Array(items))
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.skip_ignored()?;
        match self.peek() {
            Some(b'"') => self.parse_quoted_string(),
            Some(_) => self.parse_unquoted_string(),
            None => Err("xcodebuild: unexpected end of input while parsing string".to_string()),
        }
    }

    fn parse_quoted_string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut result = String::new();
        while let Some(byte) = self.next() {
            match byte {
                b'"' => return Ok(result),
                b'\\' => {
                    let escaped = self.next().ok_or_else(|| {
                        "xcodebuild: unterminated escape sequence in string".to_string()
                    })?;
                    result.push(match escaped {
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        other => other as char,
                    });
                }
                other => result.push(other as char),
            }
        }
        Err("xcodebuild: unterminated quoted string".to_string())
    }

    fn parse_unquoted_string(&mut self) -> Result<String, String> {
        let start = self.index;
        while let Some(byte) = self.peek() {
            if byte.is_ascii_whitespace() || matches!(byte, b'{' | b'}' | b'(' | b')' | b';' | b',')
            {
                break;
            }
            if byte == b'/'
                && self
                    .peek_n(1)
                    .is_some_and(|next| next == b'*' || next == b'/')
            {
                break;
            }
            self.index += 1;
        }
        if start == self.index {
            return Err("xcodebuild: expected string token".to_string());
        }
        Ok(String::from_utf8_lossy(&self.input[start..self.index]).into_owned())
    }

    fn skip_ignored(&mut self) -> Result<(), String> {
        loop {
            while self.peek().is_some_and(|byte| byte.is_ascii_whitespace()) {
                self.index += 1;
            }

            if self.peek() == Some(b'/') && self.peek_n(1) == Some(b'*') {
                self.index += 2;
                while !(self.peek() == Some(b'*') && self.peek_n(1) == Some(b'/')) {
                    if self.next().is_none() {
                        return Err("xcodebuild: unterminated block comment".to_string());
                    }
                }
                self.index += 2;
                continue;
            }

            if self.peek() == Some(b'/') && self.peek_n(1) == Some(b'/') {
                self.index += 2;
                while let Some(byte) = self.next() {
                    if byte == b'\n' {
                        break;
                    }
                }
                continue;
            }

            break;
        }

        Ok(())
    }

    fn expect(&mut self, byte: u8) -> Result<(), String> {
        self.skip_ignored()?;
        if self.try_consume(byte) {
            Ok(())
        } else {
            Err(format!("xcodebuild: expected '{}'", byte as char))
        }
    }

    fn try_consume(&mut self, byte: u8) -> bool {
        if self.peek() == Some(byte) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.index).copied()
    }

    fn peek_n(&self, offset: usize) -> Option<u8> {
        self.input.get(self.index + offset).copied()
    }

    fn next(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.index += 1;
        Some(byte)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_project_pbxproj() -> &'static str {
        r#"// !$*UTF8*$!
{
    archiveVersion = 1;
    classes = {};
    objectVersion = 56;
    objects = {
        PROJECT_DEBUG = {
            isa = XCBuildConfiguration;
            buildSettings = {
                BUILD_DIR = "$(PROJECT_DIR)/build";
            };
            name = Debug;
        };
        PROJECT_RELEASE = {
            isa = XCBuildConfiguration;
            buildSettings = {
                BUILD_DIR = "$(PROJECT_DIR)/build";
            };
            name = Release;
        };
        PROJECT_CONFIG_LIST = {
            isa = XCConfigurationList;
            buildConfigurations = (
                PROJECT_DEBUG,
                PROJECT_RELEASE,
            );
            defaultConfigurationName = Debug;
        };
        TARGET_DEBUG = {
            isa = XCBuildConfiguration;
            buildSettings = {
                PRODUCT_NAME = "$(TARGET_NAME)";
                OTHER_CFLAGS = -DHELLO_VALUE=41;
            };
            name = Debug;
        };
        TARGET_RELEASE = {
            isa = XCBuildConfiguration;
            buildSettings = {
                PRODUCT_NAME = "$(TARGET_NAME)";
                OTHER_CFLAGS = -DHELLO_VALUE=99;
            };
            name = Release;
        };
        TARGET_CONFIG_LIST = {
            isa = XCConfigurationList;
            buildConfigurations = (
                TARGET_DEBUG,
                TARGET_RELEASE,
            );
            defaultConfigurationName = Debug;
        };
        MAIN_FILE = {
            isa = PBXFileReference;
            lastKnownFileType = sourcecode.c.c;
            path = main.c;
            sourceTree = SOURCE_ROOT;
        };
        PRODUCT_FILE = {
            isa = PBXFileReference;
            explicitFileType = compiled.mach-o.executable;
            path = NativeHello;
            sourceTree = BUILT_PRODUCTS_DIR;
        };
        MAIN_BUILD_FILE = {
            isa = PBXBuildFile;
            fileRef = MAIN_FILE;
        };
        SOURCES_PHASE = {
            isa = PBXSourcesBuildPhase;
            files = (
                MAIN_BUILD_FILE,
            );
        };
        MAIN_GROUP = {
            isa = PBXGroup;
            children = (
                MAIN_FILE,
                PRODUCT_FILE,
            );
            sourceTree = "<group>";
        };
        MAIN_TARGET = {
            isa = PBXNativeTarget;
            buildConfigurationList = TARGET_CONFIG_LIST;
            buildPhases = (
                SOURCES_PHASE,
            );
            name = NativeHello;
            productName = NativeHello;
            productReference = PRODUCT_FILE;
            productType = "com.apple.product-type.tool";
        };
        ROOT_PROJECT = {
            isa = PBXProject;
            buildConfigurationList = PROJECT_CONFIG_LIST;
            mainGroup = MAIN_GROUP;
            targets = (
                MAIN_TARGET,
            );
        };
    };
    rootObject = ROOT_PROJECT;
}
"#
    }

    fn sample_source() -> &'static str {
        r#"#include <stdio.h>

int main(void) {
    printf("hello xcodebuild %d\n", HELLO_VALUE);
    return 0;
}
"#
    }

    fn create_sample_project(temp: &TempDir) -> PathBuf {
        let project_root = temp.path().join("NativeHello");
        let bundle = project_root.join("NativeHello.xcodeproj");
        fs::create_dir_all(&bundle).expect("create xcodeproj bundle");
        fs::write(bundle.join("project.pbxproj"), sample_project_pbxproj()).expect("write pbxproj");
        fs::write(project_root.join("main.c"), sample_source()).expect("write main.c");
        bundle
    }

    fn test_env(temp: &TempDir) -> BTreeMap<String, String> {
        let mut vars = BTreeMap::new();
        vars.insert(
            "BURDETTIE_XCODE_SELECT_PATH_FILE".to_string(),
            normalize_output_path(&temp.path().join("xcode-select-path")),
        );
        vars
    }

    #[test]
    fn list_reports_project_targets_and_configurations() {
        let temp = TempDir::new().expect("tempdir");
        let project = create_sample_project(&temp);
        let vars = test_env(&temp);

        let plan = plan_xcodebuild(
            &[
                "-list".to_string(),
                "-project".to_string(),
                normalize_output_path(&project),
            ],
            &vars,
            temp.path(),
        )
        .expect("plan resolves");

        match plan {
            XcodebuildPlan::Print(output) => {
                assert!(output.contains("Information about project \"NativeHello\""));
                assert!(output.contains("NativeHello"));
                assert!(output.contains("Debug"));
                assert!(output.contains("Release"));
            }
            other => panic!("unexpected plan: {other:?}"),
        }
    }

    #[test]
    fn show_build_settings_expands_product_and_paths() {
        let temp = TempDir::new().expect("tempdir");
        let project = create_sample_project(&temp);
        let vars = test_env(&temp);

        let plan = plan_xcodebuild(
            &[
                "-showBuildSettings".to_string(),
                "-project".to_string(),
                normalize_output_path(&project),
                "-target".to_string(),
                "NativeHello".to_string(),
                "-configuration".to_string(),
                "Debug".to_string(),
            ],
            &vars,
            temp.path(),
        )
        .expect("plan resolves");

        match plan {
            XcodebuildPlan::Print(output) => {
                assert!(output.contains("PRODUCT_NAME = NativeHello"));
                assert!(output.contains("CONFIGURATION = Debug"));
                assert!(output.contains("OTHER_CFLAGS = -DHELLO_VALUE=41"));
                assert!(output.contains(&format!(
                    "TARGET_BUILD_DIR = {}",
                    normalize_output_path(
                        &temp.path().join("NativeHello").join("build").join("Debug")
                    )
                )));
            }
            other => panic!("unexpected plan: {other:?}"),
        }
    }

    #[test]
    fn build_plan_uses_configured_compiler_and_output_path() {
        let temp = TempDir::new().expect("tempdir");
        let project = create_sample_project(&temp);
        let fake_compiler = temp.path().join("fake-cc");
        fs::write(&fake_compiler, b"").expect("write fake compiler");
        let mut vars = test_env(&temp);
        vars.insert("CC".to_string(), normalize_output_path(&fake_compiler));

        let plan = plan_xcodebuild(
            &[
                "build".to_string(),
                "-project".to_string(),
                normalize_output_path(&project),
            ],
            &vars,
            temp.path(),
        )
        .expect("plan resolves");

        match plan {
            XcodebuildPlan::Build(plan) => {
                assert_eq!(plan.compiler, normalize_output_path(&fake_compiler));
                assert!(plan.args.contains(&"-DHELLO_VALUE=41".to_string()));
                assert!(plan.output.ends_with("/build/Debug/NativeHello"));
            }
            other => panic!("unexpected plan: {other:?}"),
        }
    }

    #[test]
    fn compiler_selection_ignores_overlay_clang_by_default() {
        let temp = TempDir::new().expect("tempdir");
        let overlay_root = temp.path().join("overlay");
        let path_bin = temp.path().join("bin");
        let fake_cc = path_bin.join("cc");
        fs::create_dir_all(overlay_root.join("xcode-tc")).expect("create overlay toolchain dir");
        fs::create_dir_all(&path_bin).expect("create path dir");
        fs::write(overlay_root.join("xcode-tc").join("clang"), b"").expect("write overlay clang");
        fs::write(&fake_cc, b"").expect("write fake cc");

        let mut vars = test_env(&temp);
        vars.insert(
            "BURDETTIE_XCODE_TOOLS_ROOT".to_string(),
            normalize_output_path(&overlay_root),
        );
        vars.insert("PATH".to_string(), normalize_output_path(&path_bin));

        assert_eq!(
            compiler_for_build(&vars).expect("compiler resolves"),
            normalize_output_path(&fake_cc)
        );
    }

    #[test]
    fn show_build_settings_uses_symbolic_sdkroot_when_overlay_sdk_is_missing() {
        let temp = TempDir::new().expect("tempdir");
        let project = create_sample_project(&temp);
        let overlay_root = temp.path().join("overlay");
        fs::create_dir_all(overlay_root.join("xcode-dev")).expect("create overlay marker");
        let mut vars = test_env(&temp);
        vars.insert(
            "BURDETTIE_XCODE_TOOLS_ROOT".to_string(),
            normalize_output_path(&overlay_root),
        );

        let plan = plan_xcodebuild(
            &[
                "-showBuildSettings".to_string(),
                "-project".to_string(),
                normalize_output_path(&project),
                "-target".to_string(),
                "NativeHello".to_string(),
                "-configuration".to_string(),
                "Debug".to_string(),
            ],
            &vars,
            temp.path(),
        )
        .expect("plan resolves");

        match plan {
            XcodebuildPlan::Print(output) => {
                assert!(output.contains("SDKROOT = macosx"));
            }
            other => panic!("unexpected plan: {other:?}"),
        }
    }

    #[test]
    fn compiler_selection_rejects_overlay_only_path_entries() {
        let temp = TempDir::new().expect("tempdir");
        let overlay_root = temp.path().join("overlay");
        let overlay_tc = overlay_root.join("xcode-tc");
        fs::create_dir_all(&overlay_tc).expect("create overlay toolchain dir");
        fs::write(overlay_tc.join("clang"), b"").expect("write overlay clang");

        let mut vars = test_env(&temp);
        vars.insert(
            "BURDETTIE_XCODE_TOOLS_ROOT".to_string(),
            normalize_output_path(&overlay_root),
        );
        vars.insert("PATH".to_string(), normalize_output_path(&overlay_tc));

        let error =
            compiler_for_build(&vars).expect_err("overlay-only compiler discovery should fail");
        assert!(error.contains("no compiler found"));
    }

    #[test]
    fn clean_uses_project_build_dir() {
        let temp = TempDir::new().expect("tempdir");
        let project = create_sample_project(&temp);
        let vars = test_env(&temp);

        let plan = plan_xcodebuild(
            &[
                "clean".to_string(),
                "-project".to_string(),
                normalize_output_path(&project),
            ],
            &vars,
            temp.path(),
        )
        .expect("plan resolves");

        match plan {
            XcodebuildPlan::Clean { build_dir } => {
                assert_eq!(
                    build_dir,
                    normalize_output_path(&temp.path().join("NativeHello").join("build"))
                );
            }
            other => panic!("unexpected plan: {other:?}"),
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn build_plan_compiles_and_runs_sample_project() {
        let temp = TempDir::new().expect("tempdir");
        let project = create_sample_project(&temp);
        let vars = test_env(&temp);

        if compiler_for_build(&vars).is_err() {
            return;
        }

        let plan = plan_xcodebuild(
            &[
                "build".to_string(),
                "-project".to_string(),
                normalize_output_path(&project),
            ],
            &vars,
            temp.path(),
        )
        .expect("plan resolves");

        let build_plan = match plan {
            XcodebuildPlan::Build(plan) => plan,
            other => panic!("unexpected plan: {other:?}"),
        };

        assert_eq!(run_build_plan(&build_plan), 0);

        let output = Command::new(&build_plan.output)
            .output()
            .expect("run built binary");
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "hello xcodebuild 41"
        );
    }
}
