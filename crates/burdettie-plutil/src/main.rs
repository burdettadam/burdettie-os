use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use plist::{Dictionary, Value};
use serde_json::Value as JsonValue;
use std::io::{self, Cursor, Read};
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    let result = run(&args[1..]);
    process::exit(if result { 0 } else { 1 });
}

fn run(args: &[String]) -> bool {
    let mut i = 0;
    let cmd = &args[0];

    match cmd.as_str() {
        "-help" => {
            print_usage();
            return true;
        }
        "-lint" => {
            i += 1;
            let (opts, rest) = parse_options(&args[i..]);
            return cmd_lint(&rest, &opts);
        }
        "-p" => {
            i += 1;
            let (opts, rest) = parse_options(&args[i..]);
            return cmd_print(&rest, &opts);
        }
        "-convert" => {
            i += 1;
            if i >= args.len() {
                eprintln!("No format specified for -convert");
                return false;
            }
            let fmt = args[i].clone();
            i += 1;
            let (opts, rest) = parse_options(&args[i..]);
            return cmd_convert(&fmt, &rest, &opts);
        }
        "-insert" => {
            i += 1;
            if i >= args.len() {
                eprintln!("No keypath specified for -insert");
                return false;
            }
            let keypath = args[i].clone();
            i += 1;
            let (type_flag, type_value, remaining) = parse_type_and_value(&args[i..]);
            let (opts, rest) = parse_options(&remaining);
            return cmd_insert(
                &keypath,
                type_flag.as_deref(),
                type_value.as_deref(),
                &rest,
                &opts,
            );
        }
        "-replace" => {
            i += 1;
            if i >= args.len() {
                eprintln!("No keypath specified for -replace");
                return false;
            }
            let keypath = args[i].clone();
            i += 1;
            let (type_flag, type_value, remaining) = parse_type_and_value(&args[i..]);
            let (opts, rest) = parse_options(&remaining);
            return cmd_replace(
                &keypath,
                type_flag.as_deref(),
                type_value.as_deref(),
                &rest,
                &opts,
            );
        }
        "-remove" => {
            i += 1;
            if i >= args.len() {
                eprintln!("No keypath specified for -remove");
                return false;
            }
            let keypath = args[i].clone();
            i += 1;
            let (opts, rest) = parse_options(&args[i..]);
            return cmd_remove(&keypath, &rest, &opts);
        }
        "-extract" => {
            i += 1;
            if i >= args.len() {
                eprintln!("No keypath specified for -extract");
                return false;
            }
            let keypath = args[i].clone();
            i += 1;
            if i >= args.len() {
                eprintln!("No format specified for -extract");
                return false;
            }
            let fmt = args[i].clone();
            i += 1;
            let (opts, rest) = parse_options(&args[i..]);
            return cmd_extract(&keypath, &fmt, &rest, &opts);
        }
        "-type" => {
            i += 1;
            if i >= args.len() {
                eprintln!("No keypath specified for -type");
                return false;
            }
            let keypath = args[i].clone();
            i += 1;
            let (opts, rest) = parse_options(&args[i..]);
            return cmd_type(&keypath, &rest, &opts);
        }
        "-create" => {
            i += 1;
            let (fmt, rest_start) = if i < args.len() && !args[i].starts_with('-') {
                (args[i].clone(), i + 1)
            } else if i < args.len()
                && (args[i] == "xml1" || args[i] == "binary1" || args[i] == "json")
            {
                (args[i].clone(), i + 1)
            } else {
                ("xml1".to_string(), i)
            };
            let (opts, rest) = parse_options(&args[rest_start..]);
            return cmd_create(&fmt, &rest, &opts);
        }
        _ => {
            // Default: no command means -lint
            if !cmd.starts_with('-') {
                // It's a filename — default to lint
                let (opts, rest) = parse_options(args);
                return cmd_lint(&rest, &opts);
            }
            eprintln!("Unknown option: {cmd}");
            print_usage();
            return false;
        }
    }
}

#[derive(Default)]
struct Options {
    silent: bool,
    output_path: Option<String>,
    readable_json: bool,
    no_newline: bool,
    extension: Option<String>,
}

fn parse_options(args: &[String]) -> (Options, Vec<String>) {
    let mut opts = Options::default();
    let mut files = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-s" => opts.silent = true,
            "-o" => {
                i += 1;
                if i < args.len() {
                    opts.output_path = Some(args[i].clone());
                }
            }
            "-r" => opts.readable_json = true,
            "-n" => opts.no_newline = true,
            "-e" => {
                i += 1;
                if i < args.len() {
                    opts.extension = Some(args[i].clone());
                }
            }
            "--" => {
                files.extend_from_slice(&args[i + 1..]);
                break;
            }
            _ => files.push(args[i].clone()),
        }
        i += 1;
    }
    (opts, files)
}

fn parse_type_and_value(args: &[String]) -> (Option<String>, Option<String>, Vec<String>) {
    if args.is_empty() {
        return (None, None, Vec::new());
    }
    let type_flags = [
        "-bool",
        "-integer",
        "-float",
        "-string",
        "-date",
        "-data",
        "-xml",
        "-json",
        "-array",
        "-dictionary",
    ];
    if type_flags.contains(&args[0].as_str()) {
        let type_flag = args[0].clone();
        // -array and -dictionary don't take a value argument
        if type_flag == "-array" || type_flag == "-dictionary" {
            return (Some(type_flag), None, args[1..].to_vec());
        }
        if args.len() > 1 && !args[1].starts_with('-') {
            return (Some(type_flag), Some(args[1].clone()), args[2..].to_vec());
        }
        // Value might start with - (negative numbers), check if it's not a known flag
        if args.len() > 1
            && !type_flags.contains(&args[1].as_str())
            && args[1] != "-s"
            && args[1] != "-o"
            && args[1] != "-r"
            && args[1] != "-n"
            && args[1] != "-e"
        {
            return (Some(type_flag), Some(args[1].clone()), args[2..].to_vec());
        }
        return (Some(type_flag), None, args[1..].to_vec());
    }
    (None, None, args.to_vec())
}

fn read_plist(path: &str) -> Result<Value, String> {
    if path == "-" {
        let mut buf = Vec::new();
        io::stdin()
            .read_to_end(&mut buf)
            .map_err(|e| format!("Error reading stdin: {e}"))?;
        let cursor = Cursor::new(buf);
        Value::from_reader(cursor).map_err(|e| format!("{path}: Property List error: {e}"))
    } else {
        Value::from_file(path).map_err(|e| format!("{path}: Property List error: {e}"))
    }
}

fn write_output(value: &Value, format: &str, path: &str, opts: &Options) -> Result<(), String> {
    let output_path = get_output_path(path, opts);
    match format {
        "xml1" => {
            if output_path == "-" {
                value
                    .to_writer_xml(io::stdout())
                    .map_err(|e| format!("Error writing XML: {e}"))
            } else {
                value
                    .to_file_xml(&output_path)
                    .map_err(|e| format!("Error writing {output_path}: {e}"))
            }
        }
        "binary1" => {
            if output_path == "-" {
                value
                    .to_writer_binary(io::stdout())
                    .map_err(|e| format!("Error writing binary: {e}"))
            } else {
                value
                    .to_file_binary(&output_path)
                    .map_err(|e| format!("Error writing {output_path}: {e}"))
            }
        }
        "json" => {
            let json = plist_to_json(value);
            let output = if opts.readable_json {
                serde_json::to_string_pretty(&json)
            } else {
                serde_json::to_string(&json)
            }
            .map_err(|e| format!("Error serializing JSON: {e}"))?;
            if output_path == "-" {
                print!("{output}");
                if !opts.no_newline {
                    println!();
                }
            } else {
                let mut content = output;
                if !opts.no_newline {
                    content.push('\n');
                }
                std::fs::write(&output_path, content)
                    .map_err(|e| format!("Error writing {output_path}: {e}"))?;
            }
            Ok(())
        }
        _ => Err(format!("Unrecognized format: {format}")),
    }
}

fn get_output_path(input_path: &str, opts: &Options) -> String {
    if let Some(ref out) = opts.output_path {
        return out.clone();
    }
    if input_path == "-" {
        return "-".to_string();
    }
    if let Some(ref ext) = opts.extension {
        let p = Path::new(input_path);
        return p.with_extension(ext).to_string_lossy().into_owned();
    }
    input_path.to_string()
}

fn plist_to_json(value: &Value) -> JsonValue {
    match value {
        Value::String(s) => JsonValue::String(s.clone()),
        Value::Integer(i) => {
            if let Some(v) = i.as_signed() {
                JsonValue::Number(serde_json::Number::from(v))
            } else if let Some(v) = i.as_unsigned() {
                JsonValue::Number(serde_json::Number::from(v))
            } else {
                JsonValue::Number(serde_json::Number::from(0))
            }
        }
        Value::Real(f) => serde_json::Number::from_f64(*f)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        Value::Boolean(b) => JsonValue::Bool(*b),
        Value::Data(d) => JsonValue::String(BASE64.encode(d)),
        Value::Date(d) => JsonValue::String(d.to_xml_format()),
        Value::Array(arr) => JsonValue::Array(arr.iter().map(plist_to_json).collect()),
        Value::Dictionary(dict) => {
            let mut map = serde_json::Map::new();
            for (k, v) in dict.iter() {
                map.insert(k.clone(), plist_to_json(v));
            }
            JsonValue::Object(map)
        }
        Value::Uid(u) => {
            // CF$UID representation
            let mut map = serde_json::Map::new();
            map.insert(
                "CF$UID".to_string(),
                JsonValue::Number(serde_json::Number::from(u.get())),
            );
            JsonValue::Object(map)
        }
        _ => JsonValue::Null,
    }
}

fn json_to_plist(value: &JsonValue) -> Value {
    match value {
        JsonValue::Null => Value::String(String::new()),
        JsonValue::Bool(b) => Value::Boolean(*b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Integer(i.into())
            } else if let Some(u) = n.as_u64() {
                Value::Integer(u.into())
            } else if let Some(f) = n.as_f64() {
                Value::Real(f)
            } else {
                Value::Real(0.0)
            }
        }
        JsonValue::String(s) => Value::String(s.clone()),
        JsonValue::Array(arr) => Value::Array(arr.iter().map(json_to_plist).collect()),
        JsonValue::Object(obj) => {
            let mut dict = Dictionary::new();
            for (k, v) in obj {
                dict.insert(k.clone(), json_to_plist(v));
            }
            Value::Dictionary(dict)
        }
    }
}

/// Navigate a keypath and return a reference to the value at that path
fn navigate_keypath<'a>(root: &'a Value, keypath: &str) -> Option<&'a Value> {
    if keypath.is_empty() {
        return Some(root);
    }
    let parts: Vec<&str> = keypath.split('.').collect();
    let mut current = root;
    for part in &parts {
        if let Some(dict) = current.as_dictionary() {
            if let Some(v) = dict.get(*part) {
                current = v;
            } else {
                return None;
            }
        } else if let Some(arr) = current.as_array() {
            if let Ok(idx) = part.parse::<usize>() {
                if idx < arr.len() {
                    current = &arr[idx];
                } else {
                    return None;
                }
            } else {
                return None;
            }
        } else {
            return None;
        }
    }
    Some(current)
}

/// Navigate a keypath mutably, returning a mutable reference to the value
fn navigate_keypath_mut<'a>(root: &'a mut Value, keypath: &str) -> Option<&'a mut Value> {
    if keypath.is_empty() {
        return Some(root);
    }
    let parts: Vec<&str> = keypath.split('.').collect();
    let mut current = root;
    for part in &parts {
        if let Some(idx) = part.parse::<usize>().ok() {
            let arr = current.as_array_mut()?;
            if idx >= arr.len() {
                return None;
            }
            current = &mut arr[idx];
        } else {
            let dict = current.as_dictionary_mut()?;
            current = dict.get_mut(*part)?;
        }
    }
    Some(current)
}

/// Navigate to the parent of the last keypath component, returning (parent, last_key)
fn navigate_to_parent<'a>(root: &'a mut Value, keypath: &str) -> Option<(&'a mut Value, String)> {
    let parts: Vec<&str> = keypath.split('.').collect();
    if parts.is_empty() {
        return None;
    }
    let last = parts.last().unwrap().to_string();
    if parts.len() == 1 {
        return Some((root, last));
    }
    let parent_path = parts[..parts.len() - 1].join(".");
    let parent = navigate_keypath_mut(root, &parent_path)?;
    Some((parent, last))
}

fn make_value(type_flag: Option<&str>, type_value: Option<&str>) -> Result<Value, String> {
    match type_flag {
        Some("-bool") => {
            let v = type_value.unwrap_or("true");
            match v {
                "YES" | "true" | "1" => Ok(Value::Boolean(true)),
                "NO" | "false" | "0" => Ok(Value::Boolean(false)),
                _ => Err(format!("Invalid boolean value: {v}")),
            }
        }
        Some("-integer") => {
            let v = type_value.unwrap_or("0");
            let i: i64 = v.parse().map_err(|_| format!("Invalid integer: {v}"))?;
            Ok(Value::Integer(i.into()))
        }
        Some("-float") => {
            let v = type_value.unwrap_or("0.0");
            let f: f64 = v.parse().map_err(|_| format!("Invalid float: {v}"))?;
            Ok(Value::Real(f))
        }
        Some("-string") => {
            let v = type_value.unwrap_or("");
            Ok(Value::String(v.to_string()))
        }
        Some("-date") => {
            let v = type_value.ok_or("No date value specified")?;
            let date = plist::Date::from_xml_format(v).map_err(|_| format!("Invalid date: {v}"))?;
            Ok(Value::Date(date))
        }
        Some("-data") => {
            let v = type_value.unwrap_or("");
            let bytes = BASE64
                .decode(v)
                .map_err(|e| format!("Invalid base64 data: {e}"))?;
            Ok(Value::Data(bytes))
        }
        Some("-xml") => {
            let v = type_value.ok_or("No XML value specified")?;
            let cursor = Cursor::new(v.as_bytes());
            Value::from_reader_xml(cursor).map_err(|e| format!("Invalid XML plist: {e}"))
        }
        Some("-json") => {
            let v = type_value.ok_or("No JSON value specified")?;
            let json: JsonValue =
                serde_json::from_str(v).map_err(|e| format!("Invalid JSON: {e}"))?;
            Ok(json_to_plist(&json))
        }
        Some("-array") => Ok(Value::Array(Vec::new())),
        Some("-dictionary") => Ok(Value::Dictionary(Dictionary::new())),
        None => {
            // Default to string if value provided, empty string otherwise
            Ok(Value::String(type_value.unwrap_or("").to_string()))
        }
        Some(other) => Err(format!("Unknown type flag: {other}")),
    }
}

fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::String(_) => "string",
        Value::Integer(_) => "integer",
        Value::Real(_) => "real",
        Value::Boolean(_) => "bool",
        Value::Data(_) => "data",
        Value::Date(_) => "date",
        Value::Array(_) => "array",
        Value::Dictionary(_) => "dictionary",
        Value::Uid(_) => "uid",
        _ => "unknown",
    }
}

fn pretty_print(value: &Value, indent: usize) {
    let prefix = "  ".repeat(indent);
    match value {
        Value::Dictionary(dict) => {
            println!("{prefix}{{");
            for (k, v) in dict.iter() {
                print!("{prefix}  {k:?} => ");
                pretty_print_inline(v, indent + 1);
            }
            println!("{prefix}}}");
        }
        Value::Array(arr) => {
            println!("{prefix}(");
            for v in arr {
                print!("{prefix}  ");
                pretty_print_inline(v, indent + 1);
            }
            println!("{prefix})");
        }
        _ => {
            print!("{prefix}");
            pretty_print_inline(value, indent);
        }
    }
}

fn pretty_print_inline(value: &Value, indent: usize) {
    match value {
        Value::String(s) => println!("{s:?}"),
        Value::Integer(i) => {
            if let Some(v) = i.as_signed() {
                println!("{v}");
            } else if let Some(v) = i.as_unsigned() {
                println!("{v}");
            }
        }
        Value::Real(f) => println!("{f}"),
        Value::Boolean(b) => println!("{}", if *b { "1" } else { "0" }),
        Value::Data(d) => {
            let encoded = BASE64.encode(d);
            println!("<{encoded}>");
        }
        Value::Date(d) => println!("{}", d.to_xml_format()),
        Value::Uid(u) => println!("CF$UID {}", u.get()),
        Value::Dictionary(_) | Value::Array(_) => {
            println!();
            pretty_print(value, indent);
        }
        _ => println!("<unknown>"),
    }
}

// --- Commands ---

fn cmd_lint(files: &[String], opts: &Options) -> bool {
    if files.is_empty() {
        eprintln!("No files specified");
        return false;
    }
    let mut all_ok = true;
    for file in files {
        match read_plist(file) {
            Ok(_) => {
                if !opts.silent {
                    println!("{file}: OK");
                }
            }
            Err(e) => {
                eprintln!("{e}");
                all_ok = false;
            }
        }
    }
    all_ok
}

fn cmd_print(files: &[String], _opts: &Options) -> bool {
    if files.is_empty() {
        eprintln!("No files specified");
        return false;
    }
    let mut all_ok = true;
    for file in files {
        match read_plist(file) {
            Ok(value) => {
                pretty_print(&value, 0);
            }
            Err(e) => {
                eprintln!("{e}");
                all_ok = false;
            }
        }
    }
    all_ok
}

fn cmd_convert(format: &str, files: &[String], opts: &Options) -> bool {
    match format {
        "xml1" | "binary1" | "json" => {}
        _ => {
            eprintln!("Unrecognized format: {format}");
            eprintln!("Supported formats: xml1 binary1 json");
            return false;
        }
    }
    if files.is_empty() {
        eprintln!("No files specified");
        return false;
    }
    let mut all_ok = true;
    for file in files {
        match read_plist(file) {
            Ok(value) => {
                if let Err(e) = write_output(&value, format, file, opts) {
                    eprintln!("{e}");
                    all_ok = false;
                }
            }
            Err(e) => {
                eprintln!("{e}");
                all_ok = false;
            }
        }
    }
    all_ok
}

fn cmd_insert(
    keypath: &str,
    type_flag: Option<&str>,
    type_value: Option<&str>,
    files: &[String],
    opts: &Options,
) -> bool {
    if files.is_empty() {
        eprintln!("No files specified");
        return false;
    }
    let new_value = match make_value(type_flag, type_value) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            return false;
        }
    };
    let mut all_ok = true;
    for file in files {
        match read_plist(file) {
            Ok(mut root) => {
                if keypath.is_empty() {
                    // Replace entire root
                    root = new_value.clone();
                } else if let Some((parent, key)) = navigate_to_parent(&mut root, keypath) {
                    if let Some(dict) = parent.as_dictionary_mut() {
                        if dict.contains_key(&key) {
                            eprintln!("{file}: Key already exists at keypath \"{keypath}\"");
                            all_ok = false;
                            continue;
                        }
                        dict.insert(key, new_value.clone());
                    } else if let Some(arr) = parent.as_array_mut() {
                        if let Ok(idx) = key.parse::<usize>() {
                            if idx > arr.len() {
                                eprintln!(
                                    "{file}: Array index {idx} out of range for keypath \"{keypath}\""
                                );
                                all_ok = false;
                                continue;
                            }
                            arr.insert(idx, new_value.clone());
                        } else {
                            eprintln!(
                                "{file}: Cannot insert key \"{key}\" into non-dictionary container"
                            );
                            all_ok = false;
                            continue;
                        }
                    } else {
                        eprintln!("{file}: Cannot navigate to keypath \"{keypath}\"");
                        all_ok = false;
                        continue;
                    }
                } else {
                    eprintln!("{file}: Cannot navigate to keypath \"{keypath}\"");
                    all_ok = false;
                    continue;
                }
                let out_path = get_output_path(file, opts);
                if let Err(e) = write_value_to_original_format(&root, file, &out_path) {
                    eprintln!("{e}");
                    all_ok = false;
                }
            }
            Err(e) => {
                eprintln!("{e}");
                all_ok = false;
            }
        }
    }
    all_ok
}

fn cmd_replace(
    keypath: &str,
    type_flag: Option<&str>,
    type_value: Option<&str>,
    files: &[String],
    opts: &Options,
) -> bool {
    if files.is_empty() {
        eprintln!("No files specified");
        return false;
    }
    let new_value = match make_value(type_flag, type_value) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            return false;
        }
    };
    let mut all_ok = true;
    for file in files {
        match read_plist(file) {
            Ok(mut root) => {
                if keypath.is_empty() {
                    root = new_value.clone();
                } else if let Some(target) = navigate_keypath_mut(&mut root, keypath) {
                    *target = new_value.clone();
                } else {
                    eprintln!("{file}: Key does not exist at keypath \"{keypath}\"");
                    all_ok = false;
                    continue;
                }
                let out_path = get_output_path(file, opts);
                if let Err(e) = write_value_to_original_format(&root, file, &out_path) {
                    eprintln!("{e}");
                    all_ok = false;
                }
            }
            Err(e) => {
                eprintln!("{e}");
                all_ok = false;
            }
        }
    }
    all_ok
}

fn cmd_remove(keypath: &str, files: &[String], opts: &Options) -> bool {
    if files.is_empty() {
        eprintln!("No files specified");
        return false;
    }
    let mut all_ok = true;
    for file in files {
        match read_plist(file) {
            Ok(mut root) => {
                if keypath.is_empty() {
                    eprintln!("{file}: Cannot remove root");
                    all_ok = false;
                    continue;
                }
                if let Some((parent, key)) = navigate_to_parent(&mut root, keypath) {
                    if let Some(dict) = parent.as_dictionary_mut() {
                        if dict.remove(&key).is_none() {
                            eprintln!("{file}: Key does not exist at keypath \"{keypath}\"");
                            all_ok = false;
                            continue;
                        }
                    } else if let Some(arr) = parent.as_array_mut() {
                        if let Ok(idx) = key.parse::<usize>() {
                            if idx >= arr.len() {
                                eprintln!(
                                    "{file}: Array index {idx} out of range for keypath \"{keypath}\""
                                );
                                all_ok = false;
                                continue;
                            }
                            arr.remove(idx);
                        } else {
                            eprintln!("{file}: Key does not exist at keypath \"{keypath}\"");
                            all_ok = false;
                            continue;
                        }
                    } else {
                        eprintln!("{file}: Cannot navigate to keypath \"{keypath}\"");
                        all_ok = false;
                        continue;
                    }
                } else {
                    eprintln!("{file}: Cannot navigate to keypath \"{keypath}\"");
                    all_ok = false;
                    continue;
                }
                let out_path = get_output_path(file, opts);
                if let Err(e) = write_value_to_original_format(&root, file, &out_path) {
                    eprintln!("{e}");
                    all_ok = false;
                }
            }
            Err(e) => {
                eprintln!("{e}");
                all_ok = false;
            }
        }
    }
    all_ok
}

fn cmd_extract(keypath: &str, format: &str, files: &[String], opts: &Options) -> bool {
    if files.is_empty() {
        eprintln!("No files specified");
        return false;
    }
    let mut all_ok = true;
    for file in files {
        match read_plist(file) {
            Ok(root) => {
                let value = if keypath.is_empty() {
                    &root
                } else {
                    match navigate_keypath(&root, keypath) {
                        Some(v) => v,
                        None => {
                            eprintln!("{file}: Key does not exist at keypath \"{keypath}\"");
                            all_ok = false;
                            continue;
                        }
                    }
                };
                match format {
                    "raw" => {
                        let output_str = match value {
                            Value::String(s) => s.clone(),
                            Value::Integer(i) => {
                                if let Some(v) = i.as_signed() {
                                    v.to_string()
                                } else if let Some(v) = i.as_unsigned() {
                                    v.to_string()
                                } else {
                                    "0".to_string()
                                }
                            }
                            Value::Real(f) => f.to_string(),
                            Value::Boolean(b) => if *b { "true" } else { "false" }.to_string(),
                            Value::Data(d) => BASE64.encode(d),
                            Value::Date(d) => d.to_xml_format(),
                            _ => {
                                eprintln!("{file}: Cannot extract container type as raw");
                                all_ok = false;
                                continue;
                            }
                        };
                        let output_path = get_output_path(file, opts);
                        if output_path == "-" || output_path == *file {
                            print!("{output_str}");
                            if !opts.no_newline {
                                println!();
                            }
                        } else {
                            let mut content = output_str;
                            if !opts.no_newline {
                                content.push('\n');
                            }
                            if let Err(e) = std::fs::write(&output_path, content) {
                                eprintln!("Error writing {output_path}: {e}");
                                all_ok = false;
                            }
                        }
                    }
                    "xml1" | "binary1" | "json" => {
                        if let Err(e) = write_output(value, format, file, opts) {
                            eprintln!("{e}");
                            all_ok = false;
                        }
                    }
                    _ => {
                        eprintln!("Unrecognized format: {format}");
                        all_ok = false;
                    }
                }
            }
            Err(e) => {
                eprintln!("{e}");
                all_ok = false;
            }
        }
    }
    all_ok
}

fn cmd_type(keypath: &str, files: &[String], _opts: &Options) -> bool {
    if files.is_empty() {
        eprintln!("No files specified");
        return false;
    }
    let mut all_ok = true;
    for file in files {
        match read_plist(file) {
            Ok(root) => {
                let value = if keypath.is_empty() {
                    &root
                } else {
                    match navigate_keypath(&root, keypath) {
                        Some(v) => v,
                        None => {
                            eprintln!("{file}: Key does not exist at keypath \"{keypath}\"");
                            all_ok = false;
                            continue;
                        }
                    }
                };
                println!("Type is {}", value_type_name(value));
            }
            Err(e) => {
                eprintln!("{e}");
                all_ok = false;
            }
        }
    }
    all_ok
}

fn cmd_create(format: &str, files: &[String], opts: &Options) -> bool {
    if files.is_empty() {
        eprintln!("No files specified");
        return false;
    }
    let empty = Value::Dictionary(Dictionary::new());
    let mut all_ok = true;
    for file in files {
        let output_path = get_output_path(file, opts);
        if let Err(e) = match format {
            "xml1" => {
                if output_path == "-" {
                    empty.to_writer_xml(io::stdout()).map_err(|e| e.to_string())
                } else {
                    empty.to_file_xml(&output_path).map_err(|e| e.to_string())
                }
            }
            "binary1" => {
                if output_path == "-" {
                    empty
                        .to_writer_binary(io::stdout())
                        .map_err(|e| e.to_string())
                } else {
                    empty
                        .to_file_binary(&output_path)
                        .map_err(|e| e.to_string())
                }
            }
            "json" => {
                let content = if opts.readable_json { "{}\n" } else { "{}\n" };
                if output_path == "-" {
                    print!("{content}");
                    Ok(())
                } else {
                    std::fs::write(&output_path, content).map_err(|e| e.to_string())
                }
            }
            _ => {
                eprintln!("Unrecognized format: {format}");
                all_ok = false;
                continue;
            }
        } {
            eprintln!("Error creating {output_path}: {e}");
            all_ok = false;
        }
    }
    all_ok
}

/// Detect the format of a plist file and write back in the same format
fn write_value_to_original_format(
    value: &Value,
    original_path: &str,
    output_path: &str,
) -> Result<(), String> {
    // Detect format from original file contents
    let format = if original_path == "-" {
        "xml1" // Default for stdin
    } else {
        let bytes = std::fs::read(original_path).map_err(|e| format!("{original_path}: {e}"))?;
        detect_format(&bytes)
    };
    let opts = Options::default();
    write_output(value, format, output_path, &opts)
}

fn detect_format(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(b"bplist") {
        "binary1"
    } else if bytes.starts_with(b"<?xml") || bytes.starts_with(b"<!DOCTYPE") {
        "xml1"
    } else if bytes.first().map_or(false, |b| *b == b'{' || *b == b'[') {
        "json"
    } else {
        "xml1"
    }
}

fn print_usage() {
    eprintln!("Usage: plutil [command] [options] file ...");
    eprintln!("Commands:");
    eprintln!("  -help                          show this message");
    eprintln!("  -lint                          check plist syntax (default)");
    eprintln!("  -p                             pretty-print plist");
    eprintln!("  -convert fmt                   convert to format (xml1|binary1|json)");
    eprintln!("  -insert keypath -type [value]  insert value at keypath");
    eprintln!("  -replace keypath -type value   replace value at keypath");
    eprintln!("  -remove keypath                remove value at keypath");
    eprintln!("  -extract keypath fmt           extract value at keypath as format");
    eprintln!("  -type keypath                  print type of value at keypath");
    eprintln!("  -create fmt                    create empty plist in format");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  -s                             suppress normal output (silent)");
    eprintln!("  -o path                        output to path instead of input file");
    eprintln!("  -r                             readable JSON output");
    eprintln!("  -n                             suppress trailing newline");
    eprintln!("  -e extension                   change output file extension");
    eprintln!();
    eprintln!("Types for -insert and -replace:");
    eprintln!("  -bool YES|NO   -integer N   -float N   -string S");
    eprintln!("  -date DATESTR  -data BASE64 -xml XMLSTR -json JSONSTR");
    eprintln!("  -array          -dictionary");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_dir() -> PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("plutil-test-{}-{}", std::process::id(), id));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_xml_plist(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_lint_valid_xml() {
        let dir = temp_dir();
        let path = write_xml_plist(
            &dir,
            "valid.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Name</key>
    <string>Test</string>
</dict>
</plist>"#,
        );
        let result = run(&["-lint".into(), path.to_string_lossy().into()]);
        assert!(result);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_lint_invalid() {
        let dir = temp_dir();
        let path = write_xml_plist(&dir, "invalid.plist", "not a plist at all {{{");
        let result = run(&["-lint".into(), path.to_string_lossy().into()]);
        assert!(!result);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_convert_xml_to_json() {
        let dir = temp_dir();
        let path = write_xml_plist(
            &dir,
            "convert.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>count</key>
    <integer>42</integer>
</dict>
</plist>"#,
        );
        let json_path = dir.join("convert.json");
        let result = run(&[
            "-convert".into(),
            "json".into(),
            "-o".into(),
            json_path.to_string_lossy().into(),
            path.to_string_lossy().into(),
        ]);
        assert!(result);
        let content = fs::read_to_string(&json_path).unwrap();
        let parsed: JsonValue = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["count"], 42);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_convert_xml_to_binary() {
        let dir = temp_dir();
        let path = write_xml_plist(
            &dir,
            "binary.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>enabled</key>
    <true/>
</dict>
</plist>"#,
        );
        let bin_path = dir.join("binary.bin");
        let result = run(&[
            "-convert".into(),
            "binary1".into(),
            "-o".into(),
            bin_path.to_string_lossy().into(),
            path.to_string_lossy().into(),
        ]);
        assert!(result);
        let bytes = fs::read(&bin_path).unwrap();
        assert!(bytes.starts_with(b"bplist"));
        // Read it back
        let value = Value::from_file(&bin_path).unwrap();
        assert_eq!(
            value
                .as_dictionary()
                .unwrap()
                .get("enabled")
                .unwrap()
                .as_boolean(),
            Some(true)
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_insert_and_extract() {
        let dir = temp_dir();
        let path = write_xml_plist(
            &dir,
            "insert.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Name</key>
    <string>Test</string>
</dict>
</plist>"#,
        );
        // Insert a new integer key
        let result = run(&[
            "-insert".into(),
            "Age".into(),
            "-integer".into(),
            "25".into(),
            path.to_string_lossy().into(),
        ]);
        assert!(result);

        // Read it back and verify
        let value = Value::from_file(&path).unwrap();
        let dict = value.as_dictionary().unwrap();
        assert_eq!(dict.get("Age").unwrap().as_signed_integer(), Some(25));
        assert_eq!(dict.get("Name").unwrap().as_string(), Some("Test"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_replace() {
        let dir = temp_dir();
        let path = write_xml_plist(
            &dir,
            "replace.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>version</key>
    <integer>1</integer>
</dict>
</plist>"#,
        );
        let result = run(&[
            "-replace".into(),
            "version".into(),
            "-integer".into(),
            "2".into(),
            path.to_string_lossy().into(),
        ]);
        assert!(result);

        let value = Value::from_file(&path).unwrap();
        assert_eq!(
            value
                .as_dictionary()
                .unwrap()
                .get("version")
                .unwrap()
                .as_signed_integer(),
            Some(2)
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_remove() {
        let dir = temp_dir();
        let path = write_xml_plist(
            &dir,
            "remove.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>keep</key>
    <string>yes</string>
    <key>remove</key>
    <string>no</string>
</dict>
</plist>"#,
        );
        let result = run(&[
            "-remove".into(),
            "remove".into(),
            path.to_string_lossy().into(),
        ]);
        assert!(result);

        let value = Value::from_file(&path).unwrap();
        let dict = value.as_dictionary().unwrap();
        assert!(dict.get("keep").is_some());
        assert!(dict.get("remove").is_none());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_type() {
        let dir = temp_dir();
        let path = write_xml_plist(
            &dir,
            "type.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>name</key>
    <string>hello</string>
    <key>items</key>
    <array>
        <integer>1</integer>
    </array>
</dict>
</plist>"#,
        );
        let result = run(&["-type".into(), "name".into(), path.to_string_lossy().into()]);
        assert!(result);
        let result = run(&[
            "-type".into(),
            "items".into(),
            path.to_string_lossy().into(),
        ]);
        assert!(result);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_create() {
        let dir = temp_dir();
        let path = dir.join("new.plist");
        let result = run(&[
            "-create".into(),
            "xml1".into(),
            path.to_string_lossy().into(),
        ]);
        assert!(result);
        let value = Value::from_file(&path).unwrap();
        assert!(value.as_dictionary().unwrap().is_empty());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_create_json() {
        let dir = temp_dir();
        let path = dir.join("new.json");
        let result = run(&[
            "-create".into(),
            "json".into(),
            path.to_string_lossy().into(),
        ]);
        assert!(result);
        let content = fs::read_to_string(&path).unwrap();
        let parsed: JsonValue = serde_json::from_str(content.trim()).unwrap();
        assert!(parsed.is_object());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_default_lint() {
        let dir = temp_dir();
        let path = write_xml_plist(
            &dir,
            "default.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict/>
</plist>"#,
        );
        // No command specified, should default to lint
        let result = run(&[path.to_string_lossy().into()]);
        assert!(result);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_plist_to_json_roundtrip() {
        let mut dict = Dictionary::new();
        dict.insert("str".to_string(), Value::String("hello".to_string()));
        dict.insert("num".to_string(), Value::Integer(42.into()));
        dict.insert("bool".to_string(), Value::Boolean(true));
        dict.insert("real".to_string(), Value::Real(3.14));
        dict.insert(
            "arr".to_string(),
            Value::Array(vec![Value::Integer(1.into()), Value::Integer(2.into())]),
        );
        let plist_val = Value::Dictionary(dict);
        let json = plist_to_json(&plist_val);
        let back = json_to_plist(&json);
        // Verify key fields roundtrip
        let d = back.as_dictionary().unwrap();
        assert_eq!(d.get("str").unwrap().as_string(), Some("hello"));
        assert_eq!(d.get("num").unwrap().as_signed_integer(), Some(42));
        assert_eq!(d.get("bool").unwrap().as_boolean(), Some(true));
    }

    #[test]
    fn test_navigate_keypath() {
        let mut inner = Dictionary::new();
        inner.insert("key".to_string(), Value::String("value".to_string()));
        let mut root_dict = Dictionary::new();
        root_dict.insert("nested".to_string(), Value::Dictionary(inner));
        root_dict.insert(
            "arr".to_string(),
            Value::Array(vec![Value::Integer(10.into()), Value::Integer(20.into())]),
        );
        let root = Value::Dictionary(root_dict);

        assert_eq!(
            navigate_keypath(&root, "nested.key").unwrap().as_string(),
            Some("value")
        );
        assert_eq!(
            navigate_keypath(&root, "arr.0")
                .unwrap()
                .as_signed_integer(),
            Some(10)
        );
        assert_eq!(
            navigate_keypath(&root, "arr.1")
                .unwrap()
                .as_signed_integer(),
            Some(20)
        );
        assert!(navigate_keypath(&root, "nonexistent").is_none());
        assert!(navigate_keypath(&root, "arr.5").is_none());
    }

    #[test]
    fn test_make_value() {
        assert_eq!(
            make_value(Some("-bool"), Some("YES")).unwrap().as_boolean(),
            Some(true)
        );
        assert_eq!(
            make_value(Some("-bool"), Some("NO")).unwrap().as_boolean(),
            Some(false)
        );
        assert_eq!(
            make_value(Some("-integer"), Some("99"))
                .unwrap()
                .as_signed_integer(),
            Some(99)
        );
        assert_eq!(
            make_value(Some("-float"), Some("1.5")).unwrap().as_real(),
            Some(1.5)
        );
        assert_eq!(
            make_value(Some("-string"), Some("hi")).unwrap().as_string(),
            Some("hi")
        );
        assert!(make_value(Some("-array"), None)
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty());
        assert!(make_value(Some("-dictionary"), None)
            .unwrap()
            .as_dictionary()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn test_detect_format() {
        assert_eq!(detect_format(b"bplist00..."), "binary1");
        assert_eq!(detect_format(b"<?xml version=\"1.0\"..."), "xml1");
        assert_eq!(detect_format(b"<!DOCTYPE plist ..."), "xml1");
        assert_eq!(detect_format(b"{\"key\": \"value\"}"), "json");
        assert_eq!(detect_format(b"[1,2,3]"), "json");
        assert_eq!(detect_format(b"something else"), "xml1");
        assert_eq!(detect_format(b""), "xml1");
    }

    #[test]
    fn test_nested_insert() {
        let dir = temp_dir();
        let path = write_xml_plist(
            &dir,
            "nested_insert.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>settings</key>
    <dict>
        <key>theme</key>
        <string>dark</string>
    </dict>
</dict>
</plist>"#,
        );
        let result = run(&[
            "-insert".into(),
            "settings.fontSize".into(),
            "-integer".into(),
            "14".into(),
            path.to_string_lossy().into(),
        ]);
        assert!(result);

        let value = Value::from_file(&path).unwrap();
        let settings = value
            .as_dictionary()
            .unwrap()
            .get("settings")
            .unwrap()
            .as_dictionary()
            .unwrap();
        assert_eq!(
            settings.get("fontSize").unwrap().as_signed_integer(),
            Some(14)
        );
        assert_eq!(settings.get("theme").unwrap().as_string(), Some("dark"));
        fs::remove_dir_all(&dir).ok();
    }

    // ── Helper for rich test plist ──────────────────────────────────

    const RICH_PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Name</key>
	<string>BurdettieOS</string>
	<key>Version</key>
	<integer>1</integer>
	<key>Enabled</key>
	<true/>
	<key>Score</key>
	<real>99.5</real>
	<key>Tags</key>
	<array>
		<string>macho</string>
		<string>linux</string>
		<string>arm64</string>
	</array>
	<key>Settings</key>
	<dict>
		<key>Theme</key>
		<string>dark</string>
		<key>FontSize</key>
		<integer>14</integer>
		<key>Colors</key>
		<array>
			<string>red</string>
			<string>green</string>
			<string>blue</string>
		</array>
	</dict>
	<key>Data</key>
	<data>SGVsbG8=</data>
	<key>Empty</key>
	<string></string>
</dict>
</plist>"#;

    fn write_rich_plist(dir: &Path) -> PathBuf {
        write_xml_plist(dir, "rich.plist", RICH_PLIST)
    }

    // ── Error path tests ────────────────────────────────────────────

    #[test]
    fn test_lint_no_files() {
        assert!(!run(&["-lint".into()]));
    }

    #[test]
    fn test_lint_nonexistent_file() {
        assert!(!run(&["-lint".into(), "/nonexistent/path.plist".into()]));
    }

    #[test]
    fn test_print_no_files() {
        assert!(!run(&["-p".into()]));
    }

    #[test]
    fn test_convert_no_format() {
        assert!(!run(&["-convert".into()]));
    }

    #[test]
    fn test_convert_bad_format() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        assert!(!run(&[
            "-convert".into(),
            "yaml".into(),
            path.to_string_lossy().into(),
        ]));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_convert_no_files() {
        assert!(!run(&["-convert".into(), "json".into()]));
    }

    #[test]
    fn test_insert_no_keypath() {
        assert!(!run(&["-insert".into()]));
    }

    #[test]
    fn test_replace_no_keypath() {
        assert!(!run(&["-replace".into()]));
    }

    #[test]
    fn test_remove_no_keypath() {
        assert!(!run(&["-remove".into()]));
    }

    #[test]
    fn test_extract_no_keypath() {
        assert!(!run(&["-extract".into()]));
    }

    #[test]
    fn test_extract_no_format() {
        assert!(!run(&["-extract".into(), "Name".into()]));
    }

    #[test]
    fn test_type_no_keypath() {
        assert!(!run(&["-type".into()]));
    }

    #[test]
    fn test_unknown_option() {
        assert!(!run(&["-foobar".into()]));
    }

    #[test]
    fn test_insert_duplicate_key_fails() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        assert!(!run(&[
            "-insert".into(),
            "Name".into(),
            "-string".into(),
            "dupe".into(),
            path.to_string_lossy().into(),
        ]));
        // Original value preserved
        let v = Value::from_file(&path).unwrap();
        assert_eq!(
            v.as_dictionary().unwrap().get("Name").unwrap().as_string(),
            Some("BurdettieOS")
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_replace_nonexistent_key_fails() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        assert!(!run(&[
            "-replace".into(),
            "NoSuchKey".into(),
            "-string".into(),
            "x".into(),
            path.to_string_lossy().into(),
        ]));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_remove_nonexistent_key_fails() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        assert!(!run(&[
            "-remove".into(),
            "NoSuchKey".into(),
            path.to_string_lossy().into(),
        ]));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_remove_root_fails() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        // Empty keypath = remove root → error
        assert!(!run(&[
            "-remove".into(),
            "".into(),
            path.to_string_lossy().into(),
        ]));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_insert_no_files() {
        assert!(!run(&[
            "-insert".into(),
            "key".into(),
            "-string".into(),
            "val".into(),
        ]));
    }

    #[test]
    fn test_replace_no_files() {
        assert!(!run(&[
            "-replace".into(),
            "key".into(),
            "-string".into(),
            "val".into(),
        ]));
    }

    #[test]
    fn test_remove_no_files() {
        assert!(!run(&["-remove".into(), "key".into()]));
    }

    #[test]
    fn test_extract_no_files() {
        assert!(!run(&["-extract".into(), "key".into(), "raw".into()]));
    }

    #[test]
    fn test_type_no_files() {
        assert!(!run(&["-type".into(), "key".into()]));
    }

    #[test]
    fn test_create_no_files() {
        assert!(!run(&["-create".into(), "xml1".into()]));
    }

    #[test]
    fn test_create_bad_format() {
        let dir = temp_dir();
        let path = dir.join("bad.plist");
        assert!(!run(&[
            "-create".into(),
            "yaml".into(),
            path.to_string_lossy().into(),
        ]));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_extract_nonexistent_key_fails() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        assert!(!run(&[
            "-extract".into(),
            "NoSuchKey".into(),
            "raw".into(),
            path.to_string_lossy().into(),
        ]));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_type_nonexistent_key_fails() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        assert!(!run(&[
            "-type".into(),
            "NoSuchKey".into(),
            path.to_string_lossy().into(),
        ]));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_extract_bad_format() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        assert!(!run(&[
            "-extract".into(),
            "Name".into(),
            "yaml".into(),
            path.to_string_lossy().into(),
        ]));
        fs::remove_dir_all(&dir).ok();
    }

    // ── make_value exhaustive ───────────────────────────────────────

    #[test]
    fn test_make_value_bool_all_formats() {
        for (input, expected) in &[
            ("YES", true),
            ("true", true),
            ("1", true),
            ("NO", false),
            ("false", false),
            ("0", false),
        ] {
            assert_eq!(
                make_value(Some("-bool"), Some(input)).unwrap().as_boolean(),
                Some(*expected),
                "bool input '{input}' should produce {expected}"
            );
        }
    }

    #[test]
    fn test_make_value_bool_invalid() {
        assert!(make_value(Some("-bool"), Some("maybe")).is_err());
    }

    #[test]
    fn test_make_value_bool_default() {
        // No value → defaults to true
        assert_eq!(
            make_value(Some("-bool"), None).unwrap().as_boolean(),
            Some(true)
        );
    }

    #[test]
    fn test_make_value_integer_negative() {
        assert_eq!(
            make_value(Some("-integer"), Some("-42"))
                .unwrap()
                .as_signed_integer(),
            Some(-42)
        );
    }

    #[test]
    fn test_make_value_integer_zero() {
        assert_eq!(
            make_value(Some("-integer"), Some("0"))
                .unwrap()
                .as_signed_integer(),
            Some(0)
        );
    }

    #[test]
    fn test_make_value_integer_large() {
        assert_eq!(
            make_value(Some("-integer"), Some("9999999999"))
                .unwrap()
                .as_signed_integer(),
            Some(9999999999)
        );
    }

    #[test]
    fn test_make_value_integer_invalid() {
        assert!(make_value(Some("-integer"), Some("abc")).is_err());
    }

    #[test]
    fn test_make_value_integer_default() {
        assert_eq!(
            make_value(Some("-integer"), None)
                .unwrap()
                .as_signed_integer(),
            Some(0)
        );
    }

    #[test]
    fn test_make_value_float_negative() {
        assert_eq!(
            make_value(Some("-float"), Some("-3.14")).unwrap().as_real(),
            Some(-3.14)
        );
    }

    #[test]
    fn test_make_value_float_integer_like() {
        assert_eq!(
            make_value(Some("-float"), Some("5")).unwrap().as_real(),
            Some(5.0)
        );
    }

    #[test]
    fn test_make_value_float_invalid() {
        assert!(make_value(Some("-float"), Some("abc")).is_err());
    }

    #[test]
    fn test_make_value_float_default() {
        assert_eq!(
            make_value(Some("-float"), None).unwrap().as_real(),
            Some(0.0)
        );
    }

    #[test]
    fn test_make_value_string_empty() {
        assert_eq!(
            make_value(Some("-string"), Some("")).unwrap().as_string(),
            Some("")
        );
    }

    #[test]
    fn test_make_value_string_default() {
        assert_eq!(
            make_value(Some("-string"), None).unwrap().as_string(),
            Some("")
        );
    }

    #[test]
    fn test_make_value_string_with_spaces() {
        assert_eq!(
            make_value(Some("-string"), Some("hello world"))
                .unwrap()
                .as_string(),
            Some("hello world")
        );
    }

    #[test]
    fn test_make_value_date() {
        let v = make_value(Some("-date"), Some("2026-04-07T12:00:00Z")).unwrap();
        assert!(v.as_date().is_some());
    }

    #[test]
    fn test_make_value_date_invalid() {
        assert!(make_value(Some("-date"), Some("not-a-date")).is_err());
    }

    #[test]
    fn test_make_value_date_no_value() {
        assert!(make_value(Some("-date"), None).is_err());
    }

    #[test]
    fn test_make_value_data() {
        let v = make_value(Some("-data"), Some("SGVsbG8=")).unwrap();
        assert_eq!(v.as_data(), Some(b"Hello".as_slice()));
    }

    #[test]
    fn test_make_value_data_empty() {
        let v = make_value(Some("-data"), Some("")).unwrap();
        assert_eq!(v.as_data(), Some(b"".as_slice()));
    }

    #[test]
    fn test_make_value_data_default() {
        let v = make_value(Some("-data"), None).unwrap();
        assert_eq!(v.as_data(), Some(b"".as_slice()));
    }

    #[test]
    fn test_make_value_data_invalid_base64() {
        assert!(make_value(Some("-data"), Some("!!!invalid!!!")).is_err());
    }

    #[test]
    fn test_make_value_json() {
        let v = make_value(Some("-json"), Some(r#"{"a":1}"#)).unwrap();
        let d = v.as_dictionary().unwrap();
        assert_eq!(d.get("a").unwrap().as_signed_integer(), Some(1));
    }

    #[test]
    fn test_make_value_json_array() {
        let v = make_value(Some("-json"), Some("[1,2,3]")).unwrap();
        let arr = v.as_array().unwrap();
        assert_eq!(arr.len(), 3);
    }

    #[test]
    fn test_make_value_json_invalid() {
        assert!(make_value(Some("-json"), Some("{bad json")).is_err());
    }

    #[test]
    fn test_make_value_json_no_value() {
        assert!(make_value(Some("-json"), None).is_err());
    }

    #[test]
    fn test_make_value_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><string>hi</string></plist>"#;
        let v = make_value(Some("-xml"), Some(xml)).unwrap();
        assert_eq!(v.as_string(), Some("hi"));
    }

    #[test]
    fn test_make_value_xml_invalid() {
        assert!(make_value(Some("-xml"), Some("not xml")).is_err());
    }

    #[test]
    fn test_make_value_xml_no_value() {
        assert!(make_value(Some("-xml"), None).is_err());
    }

    #[test]
    fn test_make_value_no_type_with_value() {
        // No type flag → default to string
        let v = make_value(None, Some("hello")).unwrap();
        assert_eq!(v.as_string(), Some("hello"));
    }

    #[test]
    fn test_make_value_no_type_no_value() {
        let v = make_value(None, None).unwrap();
        assert_eq!(v.as_string(), Some(""));
    }

    #[test]
    fn test_make_value_unknown_type() {
        assert!(make_value(Some("-foobar"), None).is_err());
    }

    // ── value_type_name exhaustive ──────────────────────────────────

    #[test]
    fn test_value_type_name_all() {
        assert_eq!(value_type_name(&Value::String("x".into())), "string");
        assert_eq!(value_type_name(&Value::Integer(1.into())), "integer");
        assert_eq!(value_type_name(&Value::Real(1.0)), "real");
        assert_eq!(value_type_name(&Value::Boolean(true)), "bool");
        assert_eq!(value_type_name(&Value::Data(vec![1])), "data");
        assert_eq!(
            value_type_name(&Value::Date(
                plist::Date::from_xml_format("2026-01-01T00:00:00Z").unwrap()
            )),
            "date"
        );
        assert_eq!(value_type_name(&Value::Array(vec![])), "array");
        assert_eq!(
            value_type_name(&Value::Dictionary(Dictionary::new())),
            "dictionary"
        );
        assert_eq!(value_type_name(&Value::Uid(plist::Uid::new(42))), "uid");
    }

    // ── plist ↔ JSON conversion thorough ────────────────────────────

    #[test]
    fn test_plist_to_json_data() {
        let v = Value::Data(b"Hello".to_vec());
        let j = plist_to_json(&v);
        assert_eq!(j.as_str(), Some("SGVsbG8="));
    }

    #[test]
    fn test_plist_to_json_date() {
        let d = plist::Date::from_xml_format("2026-04-07T12:00:00Z").unwrap();
        let j = plist_to_json(&Value::Date(d));
        assert_eq!(j.as_str(), Some("2026-04-07T12:00:00Z"));
    }

    #[test]
    fn test_plist_to_json_uid() {
        let j = plist_to_json(&Value::Uid(plist::Uid::new(99)));
        assert_eq!(j["CF$UID"], 99);
    }

    #[test]
    fn test_plist_to_json_nested_array() {
        let v = Value::Array(vec![
            Value::String("a".into()),
            Value::Integer(1.into()),
            Value::Boolean(false),
        ]);
        let j = plist_to_json(&v);
        let arr = j.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0], "a");
        assert_eq!(arr[1], 1);
        assert_eq!(arr[2], false);
    }

    #[test]
    fn test_plist_to_json_real_nan() {
        // NaN can't be represented in JSON → Null
        let j = plist_to_json(&Value::Real(f64::NAN));
        assert!(j.is_null());
    }

    #[test]
    fn test_json_to_plist_null() {
        let j = JsonValue::Null;
        let v = json_to_plist(&j);
        assert_eq!(v.as_string(), Some(""));
    }

    #[test]
    fn test_json_to_plist_float() {
        let j = serde_json::json!(3.14);
        let v = json_to_plist(&j);
        assert!((v.as_real().unwrap() - 3.14).abs() < 1e-10);
    }

    #[test]
    fn test_json_to_plist_nested() {
        let j = serde_json::json!({"arr": [1, "two", true], "obj": {"k": "v"}});
        let v = json_to_plist(&j);
        let d = v.as_dictionary().unwrap();
        let arr = d.get("arr").unwrap().as_array().unwrap();
        assert_eq!(arr.len(), 3);
        let inner = d.get("obj").unwrap().as_dictionary().unwrap();
        assert_eq!(inner.get("k").unwrap().as_string(), Some("v"));
    }

    // ── Keypath navigation thorough ─────────────────────────────────

    #[test]
    fn test_navigate_keypath_empty() {
        let root = Value::String("hello".into());
        assert_eq!(
            navigate_keypath(&root, "").unwrap().as_string(),
            Some("hello")
        );
    }

    #[test]
    fn test_navigate_keypath_into_string_fails() {
        let root = Value::String("hello".into());
        assert!(navigate_keypath(&root, "child").is_none());
    }

    #[test]
    fn test_navigate_keypath_array_non_numeric_fails() {
        let root = Value::Array(vec![Value::Integer(1.into())]);
        assert!(navigate_keypath(&root, "abc").is_none());
    }

    #[test]
    fn test_navigate_keypath_deep() {
        let mut l3 = Dictionary::new();
        l3.insert("val".to_string(), Value::Integer(42.into()));
        let mut l2 = Dictionary::new();
        l2.insert("l3".to_string(), Value::Dictionary(l3));
        let mut l1 = Dictionary::new();
        l1.insert("l2".to_string(), Value::Dictionary(l2));
        let root = Value::Dictionary(l1);
        assert_eq!(
            navigate_keypath(&root, "l2.l3.val")
                .unwrap()
                .as_signed_integer(),
            Some(42)
        );
    }

    #[test]
    fn test_navigate_keypath_mut_modify() {
        let mut root = {
            let mut d = Dictionary::new();
            d.insert("key".to_string(), Value::Integer(1.into()));
            Value::Dictionary(d)
        };
        let v = navigate_keypath_mut(&mut root, "key").unwrap();
        *v = Value::Integer(99.into());
        assert_eq!(
            root.as_dictionary()
                .unwrap()
                .get("key")
                .unwrap()
                .as_signed_integer(),
            Some(99)
        );
    }

    #[test]
    fn test_navigate_keypath_mut_array() {
        let mut root = Value::Array(vec![Value::String("a".into()), Value::String("b".into())]);
        let v = navigate_keypath_mut(&mut root, "1").unwrap();
        *v = Value::String("B".into());
        assert_eq!(root.as_array().unwrap()[1].as_string(), Some("B"));
    }

    #[test]
    fn test_navigate_keypath_mut_out_of_bounds() {
        let mut root = Value::Array(vec![Value::Integer(1.into())]);
        assert!(navigate_keypath_mut(&mut root, "5").is_none());
    }

    #[test]
    fn test_navigate_keypath_mut_nonexistent() {
        let mut root = Value::Dictionary(Dictionary::new());
        assert!(navigate_keypath_mut(&mut root, "nope").is_none());
    }

    #[test]
    fn test_navigate_to_parent_single_key() {
        let mut root = {
            let mut d = Dictionary::new();
            d.insert("k".to_string(), Value::Integer(1.into()));
            Value::Dictionary(d)
        };
        let (parent, key) = navigate_to_parent(&mut root, "k").unwrap();
        assert_eq!(key, "k");
        assert!(parent.as_dictionary().is_some());
    }

    #[test]
    fn test_navigate_to_parent_nested() {
        let mut root = {
            let mut inner = Dictionary::new();
            inner.insert("v".to_string(), Value::Boolean(true));
            let mut outer = Dictionary::new();
            outer.insert("inner".to_string(), Value::Dictionary(inner));
            Value::Dictionary(outer)
        };
        let (parent, key) = navigate_to_parent(&mut root, "inner.v").unwrap();
        assert_eq!(key, "v");
        assert!(parent.as_dictionary().is_some());
    }

    // ── parse_type_and_value ────────────────────────────────────────

    #[test]
    fn test_parse_type_and_value_empty() {
        let (t, v, rest) = parse_type_and_value(&[]);
        assert!(t.is_none());
        assert!(v.is_none());
        assert!(rest.is_empty());
    }

    #[test]
    fn test_parse_type_and_value_no_type() {
        let args: Vec<String> = vec!["file.plist".into()];
        let (t, v, rest) = parse_type_and_value(&args);
        assert!(t.is_none());
        assert!(v.is_none());
        assert_eq!(rest, vec!["file.plist"]);
    }

    #[test]
    fn test_parse_type_and_value_array_no_value() {
        let args: Vec<String> = vec!["-array".into(), "file.plist".into()];
        let (t, v, rest) = parse_type_and_value(&args);
        assert_eq!(t.as_deref(), Some("-array"));
        assert!(v.is_none());
        assert_eq!(rest, vec!["file.plist"]);
    }

    #[test]
    fn test_parse_type_and_value_dictionary_no_value() {
        let args: Vec<String> = vec!["-dictionary".into(), "file.plist".into()];
        let (t, v, rest) = parse_type_and_value(&args);
        assert_eq!(t.as_deref(), Some("-dictionary"));
        assert!(v.is_none());
        assert_eq!(rest, vec!["file.plist"]);
    }

    #[test]
    fn test_parse_type_and_value_string_with_value() {
        let args: Vec<String> = vec!["-string".into(), "hello".into(), "file.plist".into()];
        let (t, v, rest) = parse_type_and_value(&args);
        assert_eq!(t.as_deref(), Some("-string"));
        assert_eq!(v.as_deref(), Some("hello"));
        assert_eq!(rest, vec!["file.plist"]);
    }

    #[test]
    fn test_parse_type_and_value_negative_number() {
        let args: Vec<String> = vec!["-integer".into(), "-42".into(), "file.plist".into()];
        let (t, v, rest) = parse_type_and_value(&args);
        assert_eq!(t.as_deref(), Some("-integer"));
        assert_eq!(v.as_deref(), Some("-42"));
        assert_eq!(rest, vec!["file.plist"]);
    }

    // ── parse_options ───────────────────────────────────────────────

    #[test]
    fn test_parse_options_all_flags() {
        let args: Vec<String> = vec![
            "-s".into(),
            "-r".into(),
            "-n".into(),
            "-o".into(),
            "/tmp/out.plist".into(),
            "-e".into(),
            "json".into(),
            "file.plist".into(),
        ];
        let (opts, files) = parse_options(&args);
        assert!(opts.silent);
        assert!(opts.readable_json);
        assert!(opts.no_newline);
        assert_eq!(opts.output_path.as_deref(), Some("/tmp/out.plist"));
        assert_eq!(opts.extension.as_deref(), Some("json"));
        assert_eq!(files, vec!["file.plist"]);
    }

    #[test]
    fn test_parse_options_double_dash() {
        let args: Vec<String> = vec!["--".into(), "-s".into(), "-r".into()];
        let (opts, files) = parse_options(&args);
        // -s and -r after -- are treated as filenames
        assert!(!opts.silent);
        assert!(!opts.readable_json);
        assert_eq!(files, vec!["-s", "-r"]);
    }

    #[test]
    fn test_parse_options_empty() {
        let (opts, files) = parse_options(&[]);
        assert!(!opts.silent);
        assert!(opts.output_path.is_none());
        assert!(files.is_empty());
    }

    // ── get_output_path ─────────────────────────────────────────────

    #[test]
    fn test_get_output_path_explicit() {
        let opts = Options {
            output_path: Some("/tmp/out.plist".into()),
            ..Options::default()
        };
        assert_eq!(get_output_path("input.plist", &opts), "/tmp/out.plist");
    }

    #[test]
    fn test_get_output_path_extension() {
        let opts = Options {
            extension: Some("json".into()),
            ..Options::default()
        };
        assert_eq!(get_output_path("input.plist", &opts), "input.json");
    }

    #[test]
    fn test_get_output_path_stdin() {
        let opts = Options::default();
        assert_eq!(get_output_path("-", &opts), "-");
    }

    #[test]
    fn test_get_output_path_default() {
        let opts = Options::default();
        assert_eq!(get_output_path("file.plist", &opts), "file.plist");
    }

    // ── Binary plist roundtrip ──────────────────────────────────────

    #[test]
    fn test_xml_to_binary_to_xml_roundtrip() {
        let dir = temp_dir();
        let xml_path = write_rich_plist(&dir);
        let bin_path = dir.join("round.bplist");
        let xml2_path = dir.join("round.xml");

        // XML → binary
        assert!(run(&[
            "-convert".into(),
            "binary1".into(),
            "-o".into(),
            bin_path.to_string_lossy().into(),
            xml_path.to_string_lossy().into(),
        ]));
        assert!(fs::read(&bin_path).unwrap().starts_with(b"bplist"));

        // binary → XML
        assert!(run(&[
            "-convert".into(),
            "xml1".into(),
            "-o".into(),
            xml2_path.to_string_lossy().into(),
            bin_path.to_string_lossy().into(),
        ]));

        // Verify final XML has same data
        let v = Value::from_file(&xml2_path).unwrap();
        let d = v.as_dictionary().unwrap();
        assert_eq!(d.get("Name").unwrap().as_string(), Some("BurdettieOS"));
        assert_eq!(d.get("Version").unwrap().as_signed_integer(), Some(1));
        assert_eq!(d.get("Enabled").unwrap().as_boolean(), Some(true));
        assert_eq!(d.get("Tags").unwrap().as_array().unwrap().len(), 3);
        fs::remove_dir_all(&dir).ok();
    }

    // ── JSON plist roundtrip ────────────────────────────────────────

    #[test]
    fn test_xml_to_json_to_xml_roundtrip() {
        let dir = temp_dir();
        let xml_path = write_xml_plist(
            &dir,
            "jround.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>name</key>
    <string>test</string>
    <key>count</key>
    <integer>7</integer>
    <key>flag</key>
    <true/>
</dict>
</plist>"#,
        );
        let json_path = dir.join("jround.json");

        assert!(run(&[
            "-convert".into(),
            "json".into(),
            "-o".into(),
            json_path.to_string_lossy().into(),
            xml_path.to_string_lossy().into(),
        ]));

        // Verify JSON
        let jstr = fs::read_to_string(&json_path).unwrap();
        let j: JsonValue = serde_json::from_str(&jstr).unwrap();
        assert_eq!(j["name"], "test");
        assert_eq!(j["count"], 7);
        assert_eq!(j["flag"], true);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_convert_json_readable() {
        let dir = temp_dir();
        let xml_path = write_xml_plist(
            &dir,
            "readable.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>k</key>
    <string>v</string>
</dict>
</plist>"#,
        );
        let json_path = dir.join("readable.json");
        assert!(run(&[
            "-convert".into(),
            "json".into(),
            "-r".into(),
            "-o".into(),
            json_path.to_string_lossy().into(),
            xml_path.to_string_lossy().into(),
        ]));
        let content = fs::read_to_string(&json_path).unwrap();
        // Readable JSON should have indentation
        assert!(content.contains("  "), "expected indented JSON");
        fs::remove_dir_all(&dir).ok();
    }

    // ── Array operations ────────────────────────────────────────────

    #[test]
    fn test_insert_into_array_at_index() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        // Insert at index 1 in Tags array
        assert!(run(&[
            "-insert".into(),
            "Tags.1".into(),
            "-string".into(),
            "inserted".into(),
            path.to_string_lossy().into(),
        ]));
        let v = Value::from_file(&path).unwrap();
        let tags = v
            .as_dictionary()
            .unwrap()
            .get("Tags")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(tags.len(), 4);
        assert_eq!(tags[0].as_string(), Some("macho"));
        assert_eq!(tags[1].as_string(), Some("inserted"));
        assert_eq!(tags[2].as_string(), Some("linux"));
        assert_eq!(tags[3].as_string(), Some("arm64"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_insert_at_array_end() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        // Insert at index 3 (end of Tags array of len 3)
        assert!(run(&[
            "-insert".into(),
            "Tags.3".into(),
            "-string".into(),
            "appended".into(),
            path.to_string_lossy().into(),
        ]));
        let v = Value::from_file(&path).unwrap();
        let tags = v
            .as_dictionary()
            .unwrap()
            .get("Tags")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(tags.len(), 4);
        assert_eq!(tags[3].as_string(), Some("appended"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_insert_at_array_out_of_bounds() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        // Index 99 is way out of bounds
        assert!(!run(&[
            "-insert".into(),
            "Tags.99".into(),
            "-string".into(),
            "bad".into(),
            path.to_string_lossy().into(),
        ]));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_remove_from_array() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        // Remove index 1 ("linux") from Tags
        assert!(run(&[
            "-remove".into(),
            "Tags.1".into(),
            path.to_string_lossy().into(),
        ]));
        let v = Value::from_file(&path).unwrap();
        let tags = v
            .as_dictionary()
            .unwrap()
            .get("Tags")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].as_string(), Some("macho"));
        assert_eq!(tags[1].as_string(), Some("arm64"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_remove_array_out_of_bounds() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        assert!(!run(&[
            "-remove".into(),
            "Tags.99".into(),
            path.to_string_lossy().into(),
        ]));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_replace_array_element() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        assert!(run(&[
            "-replace".into(),
            "Tags.0".into(),
            "-string".into(),
            "MACHO".into(),
            path.to_string_lossy().into(),
        ]));
        let v = Value::from_file(&path).unwrap();
        let tags = v
            .as_dictionary()
            .unwrap()
            .get("Tags")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(tags[0].as_string(), Some("MACHO"));
        fs::remove_dir_all(&dir).ok();
    }

    // ── Extract raw for each type ───────────────────────────────────

    #[test]
    fn test_extract_raw_string() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        let out = dir.join("name.txt");
        assert!(run(&[
            "-extract".into(),
            "Name".into(),
            "raw".into(),
            "-o".into(),
            out.to_string_lossy().into(),
            path.to_string_lossy().into(),
        ]));
        let content = fs::read_to_string(&out).unwrap();
        assert_eq!(content.trim(), "BurdettieOS");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_extract_raw_integer() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        let out = dir.join("ver.txt");
        assert!(run(&[
            "-extract".into(),
            "Version".into(),
            "raw".into(),
            "-o".into(),
            out.to_string_lossy().into(),
            path.to_string_lossy().into(),
        ]));
        assert_eq!(fs::read_to_string(&out).unwrap().trim(), "1");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_extract_raw_bool() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        let out = dir.join("en.txt");
        assert!(run(&[
            "-extract".into(),
            "Enabled".into(),
            "raw".into(),
            "-o".into(),
            out.to_string_lossy().into(),
            path.to_string_lossy().into(),
        ]));
        assert_eq!(fs::read_to_string(&out).unwrap().trim(), "true");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_extract_raw_real() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        let out = dir.join("score.txt");
        assert!(run(&[
            "-extract".into(),
            "Score".into(),
            "raw".into(),
            "-o".into(),
            out.to_string_lossy().into(),
            path.to_string_lossy().into(),
        ]));
        assert_eq!(fs::read_to_string(&out).unwrap().trim(), "99.5");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_extract_raw_data() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        let out = dir.join("data.txt");
        assert!(run(&[
            "-extract".into(),
            "Data".into(),
            "raw".into(),
            "-o".into(),
            out.to_string_lossy().into(),
            path.to_string_lossy().into(),
        ]));
        assert_eq!(fs::read_to_string(&out).unwrap().trim(), "SGVsbG8=");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_extract_raw_container_fails() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        // Extracting a dictionary as raw should fail
        assert!(!run(&[
            "-extract".into(),
            "Settings".into(),
            "raw".into(),
            path.to_string_lossy().into(),
        ]));
        // Extracting an array as raw should fail
        assert!(!run(&[
            "-extract".into(),
            "Tags".into(),
            "raw".into(),
            path.to_string_lossy().into(),
        ]));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_extract_as_json() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        let out = dir.join("settings.json");
        assert!(run(&[
            "-extract".into(),
            "Settings".into(),
            "json".into(),
            "-o".into(),
            out.to_string_lossy().into(),
            path.to_string_lossy().into(),
        ]));
        let j: JsonValue = serde_json::from_str(&fs::read_to_string(&out).unwrap()).unwrap();
        assert_eq!(j["Theme"], "dark");
        assert_eq!(j["FontSize"], 14);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_extract_as_xml() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        let out = dir.join("settings.plist");
        assert!(run(&[
            "-extract".into(),
            "Settings".into(),
            "xml1".into(),
            "-o".into(),
            out.to_string_lossy().into(),
            path.to_string_lossy().into(),
        ]));
        let v = Value::from_file(&out).unwrap();
        assert_eq!(
            v.as_dictionary().unwrap().get("Theme").unwrap().as_string(),
            Some("dark")
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_extract_array_element() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        let out = dir.join("tag0.txt");
        assert!(run(&[
            "-extract".into(),
            "Tags.0".into(),
            "raw".into(),
            "-o".into(),
            out.to_string_lossy().into(),
            path.to_string_lossy().into(),
        ]));
        assert_eq!(fs::read_to_string(&out).unwrap().trim(), "macho");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_extract_nested() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        let out = dir.join("theme.txt");
        assert!(run(&[
            "-extract".into(),
            "Settings.Theme".into(),
            "raw".into(),
            "-o".into(),
            out.to_string_lossy().into(),
            path.to_string_lossy().into(),
        ]));
        assert_eq!(fs::read_to_string(&out).unwrap().trim(), "dark");
        fs::remove_dir_all(&dir).ok();
    }

    // ── Type queries for each value type ────────────────────────────

    #[test]
    fn test_type_all_types() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        // Each should succeed (type name verified by value_type_name unit test)
        for key in &[
            "Name", "Version", "Enabled", "Score", "Tags", "Settings", "Data",
        ] {
            assert!(
                run(&["-type".into(), (*key).into(), path.to_string_lossy().into(),]),
                "type query failed for key {key}"
            );
        }
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_type_root() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        // Empty keypath → root type = dictionary
        assert!(run(&[
            "-type".into(),
            "".into(),
            path.to_string_lossy().into(),
        ]));
        fs::remove_dir_all(&dir).ok();
    }

    // ── Deep nesting ────────────────────────────────────────────────

    #[test]
    fn test_deep_nested_operations() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);

        // Insert into Settings.Colors array
        assert!(run(&[
            "-insert".into(),
            "Settings.Colors.1".into(),
            "-string".into(),
            "yellow".into(),
            path.to_string_lossy().into(),
        ]));

        // Extract the 3-level deep value
        let out = dir.join("deep.txt");
        assert!(run(&[
            "-extract".into(),
            "Settings.Colors.1".into(),
            "raw".into(),
            "-o".into(),
            out.to_string_lossy().into(),
            path.to_string_lossy().into(),
        ]));
        assert_eq!(fs::read_to_string(&out).unwrap().trim(), "yellow");

        // Replace deep value
        assert!(run(&[
            "-replace".into(),
            "Settings.Colors.1".into(),
            "-string".into(),
            "YELLOW".into(),
            path.to_string_lossy().into(),
        ]));
        let v = Value::from_file(&path).unwrap();
        let colors = v
            .as_dictionary()
            .unwrap()
            .get("Settings")
            .unwrap()
            .as_dictionary()
            .unwrap()
            .get("Colors")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(colors[1].as_string(), Some("YELLOW"));

        // Remove deep value
        assert!(run(&[
            "-remove".into(),
            "Settings.Colors.1".into(),
            path.to_string_lossy().into(),
        ]));
        let v = Value::from_file(&path).unwrap();
        let colors = v
            .as_dictionary()
            .unwrap()
            .get("Settings")
            .unwrap()
            .as_dictionary()
            .unwrap()
            .get("Colors")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(colors.len(), 3); // red, green, blue (yellow removed)

        fs::remove_dir_all(&dir).ok();
    }

    // ── Create formats ──────────────────────────────────────────────

    #[test]
    fn test_create_binary() {
        let dir = temp_dir();
        let path = dir.join("new.bplist");
        assert!(run(&[
            "-create".into(),
            "binary1".into(),
            path.to_string_lossy().into(),
        ]));
        let bytes = fs::read(&path).unwrap();
        assert!(bytes.starts_with(b"bplist"));
        let v = Value::from_file(&path).unwrap();
        assert!(v.as_dictionary().unwrap().is_empty());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_create_default_xml1() {
        let dir = temp_dir();
        let path = dir.join("default_fmt.plist");
        // -create xml1 explicitly
        assert!(run(&[
            "-create".into(),
            "xml1".into(),
            path.to_string_lossy().into(),
        ]));
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("<?xml"));
        fs::remove_dir_all(&dir).ok();
    }

    // ── Insert various types ────────────────────────────────────────

    #[test]
    fn test_insert_bool() {
        let dir = temp_dir();
        let path = write_xml_plist(
            &dir,
            "bool.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict/></plist>"#,
        );
        assert!(run(&[
            "-insert".into(),
            "flag".into(),
            "-bool".into(),
            "YES".into(),
            path.to_string_lossy().into(),
        ]));
        let v = Value::from_file(&path).unwrap();
        assert_eq!(
            v.as_dictionary().unwrap().get("flag").unwrap().as_boolean(),
            Some(true)
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_insert_float() {
        let dir = temp_dir();
        let path = write_xml_plist(
            &dir,
            "float.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict/></plist>"#,
        );
        assert!(run(&[
            "-insert".into(),
            "pi".into(),
            "-float".into(),
            "3.14159".into(),
            path.to_string_lossy().into(),
        ]));
        let v = Value::from_file(&path).unwrap();
        let f = v
            .as_dictionary()
            .unwrap()
            .get("pi")
            .unwrap()
            .as_real()
            .unwrap();
        assert!((f - 3.14159).abs() < 1e-10);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_insert_data() {
        let dir = temp_dir();
        let path = write_xml_plist(
            &dir,
            "data.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict/></plist>"#,
        );
        assert!(run(&[
            "-insert".into(),
            "blob".into(),
            "-data".into(),
            "AQID".into(), // [1, 2, 3]
            path.to_string_lossy().into(),
        ]));
        let v = Value::from_file(&path).unwrap();
        assert_eq!(
            v.as_dictionary().unwrap().get("blob").unwrap().as_data(),
            Some(vec![1u8, 2, 3].as_slice())
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_insert_dictionary() {
        let dir = temp_dir();
        let path = write_xml_plist(
            &dir,
            "dict.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict/></plist>"#,
        );
        assert!(run(&[
            "-insert".into(),
            "sub".into(),
            "-dictionary".into(),
            path.to_string_lossy().into(),
        ]));
        let v = Value::from_file(&path).unwrap();
        assert!(v
            .as_dictionary()
            .unwrap()
            .get("sub")
            .unwrap()
            .as_dictionary()
            .unwrap()
            .is_empty());

        // Insert into the new sub-dictionary
        assert!(run(&[
            "-insert".into(),
            "sub.key".into(),
            "-string".into(),
            "val".into(),
            path.to_string_lossy().into(),
        ]));
        let v = Value::from_file(&path).unwrap();
        assert_eq!(
            v.as_dictionary()
                .unwrap()
                .get("sub")
                .unwrap()
                .as_dictionary()
                .unwrap()
                .get("key")
                .unwrap()
                .as_string(),
            Some("val")
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_insert_array_then_elements() {
        let dir = temp_dir();
        let path = write_xml_plist(
            &dir,
            "arr.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict/></plist>"#,
        );
        // Insert empty array
        assert!(run(&[
            "-insert".into(),
            "items".into(),
            "-array".into(),
            path.to_string_lossy().into(),
        ]));
        // Insert elements
        assert!(run(&[
            "-insert".into(),
            "items.0".into(),
            "-integer".into(),
            "10".into(),
            path.to_string_lossy().into(),
        ]));
        assert!(run(&[
            "-insert".into(),
            "items.1".into(),
            "-integer".into(),
            "20".into(),
            path.to_string_lossy().into(),
        ]));
        assert!(run(&[
            "-insert".into(),
            "items.0".into(),
            "-integer".into(),
            "5".into(),
            path.to_string_lossy().into(),
        ]));
        let v = Value::from_file(&path).unwrap();
        let items = v
            .as_dictionary()
            .unwrap()
            .get("items")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].as_signed_integer(), Some(5));
        assert_eq!(items[1].as_signed_integer(), Some(10));
        assert_eq!(items[2].as_signed_integer(), Some(20));
        fs::remove_dir_all(&dir).ok();
    }

    // ── Navigate into bad path ──────────────────────────────────────

    #[test]
    fn test_insert_into_scalar_path_fails() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        // Name is a string, can't insert a child
        assert!(!run(&[
            "-insert".into(),
            "Name.child".into(),
            "-string".into(),
            "x".into(),
            path.to_string_lossy().into(),
        ]));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_replace_through_nonexistent_path() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        assert!(!run(&[
            "-replace".into(),
            "Nonexist.child".into(),
            "-string".into(),
            "x".into(),
            path.to_string_lossy().into(),
        ]));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_remove_through_nonexistent_path() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        assert!(!run(&[
            "-remove".into(),
            "Nonexist.child".into(),
            path.to_string_lossy().into(),
        ]));
        fs::remove_dir_all(&dir).ok();
    }

    // ── Insert key into non-dict container ──────────────────────────

    #[test]
    fn test_insert_named_key_into_array_fails() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        // Tags is an array — can't insert a named key
        assert!(!run(&[
            "-insert".into(),
            "Tags.name".into(),
            "-string".into(),
            "x".into(),
            path.to_string_lossy().into(),
        ]));
        fs::remove_dir_all(&dir).ok();
    }

    // ── Output with -e extension ────────────────────────────────────

    #[test]
    fn test_convert_with_extension_option() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        assert!(run(&[
            "-convert".into(),
            "json".into(),
            "-e".into(),
            "json".into(),
            path.to_string_lossy().into(),
        ]));
        let json_path = dir.join("rich.json");
        assert!(json_path.exists());
        let j: JsonValue = serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
        assert_eq!(j["Name"], "BurdettieOS");
        fs::remove_dir_all(&dir).ok();
    }

    // ── Multiple files ──────────────────────────────────────────────

    #[test]
    fn test_lint_multiple_files_all_valid() {
        let dir = temp_dir();
        let p1 = write_xml_plist(
            &dir,
            "a.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict/></plist>"#,
        );
        let p2 = write_xml_plist(
            &dir,
            "b.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict/></plist>"#,
        );
        assert!(run(&[
            "-lint".into(),
            p1.to_string_lossy().into(),
            p2.to_string_lossy().into(),
        ]));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_lint_multiple_files_one_invalid() {
        let dir = temp_dir();
        let p1 = write_xml_plist(
            &dir,
            "good.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict/></plist>"#,
        );
        let p2 = dir.join("bad.plist");
        fs::write(
            &p2,
            &[0x62, 0x70, 0x6c, 0x69, 0x73, 0x74, 0x30, 0x30, 0xff, 0xff],
        )
        .unwrap(); // corrupt bplist header
        let result = run(&[
            "-lint".into(),
            p1.to_string_lossy().into(),
            p2.to_string_lossy().into(),
        ]);
        assert!(!result, "should fail when any file is invalid");
        fs::remove_dir_all(&dir).ok();
    }

    // ── Format preservation (write back in original format) ─────────

    #[test]
    fn test_insert_preserves_binary_format() {
        let dir = temp_dir();
        // Create a binary plist
        let path = dir.join("preserve.bplist");
        let mut d = Dictionary::new();
        d.insert("existing".to_string(), Value::Boolean(true));
        Value::Dictionary(d).to_file_binary(&path).unwrap();

        assert!(run(&[
            "-insert".into(),
            "newkey".into(),
            "-string".into(),
            "hello".into(),
            path.to_string_lossy().into(),
        ]));

        // Should still be binary
        let bytes = fs::read(&path).unwrap();
        assert!(
            bytes.starts_with(b"bplist"),
            "should preserve binary format after insert"
        );
        let v = Value::from_file(&path).unwrap();
        assert_eq!(
            v.as_dictionary()
                .unwrap()
                .get("newkey")
                .unwrap()
                .as_string(),
            Some("hello")
        );
        assert_eq!(
            v.as_dictionary()
                .unwrap()
                .get("existing")
                .unwrap()
                .as_boolean(),
            Some(true)
        );
        fs::remove_dir_all(&dir).ok();
    }

    // ── Replace with different type ─────────────────────────────────

    #[test]
    fn test_replace_changes_type() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        // Replace integer Version with string
        assert!(run(&[
            "-replace".into(),
            "Version".into(),
            "-string".into(),
            "2.0-beta".into(),
            path.to_string_lossy().into(),
        ]));
        let v = Value::from_file(&path).unwrap();
        assert_eq!(
            v.as_dictionary()
                .unwrap()
                .get("Version")
                .unwrap()
                .as_string(),
            Some("2.0-beta")
        );
        fs::remove_dir_all(&dir).ok();
    }

    // ── Lint with -s silent ─────────────────────────────────────────

    #[test]
    fn test_lint_silent_valid() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        // Should succeed with no output (we just test return value)
        assert!(run(&[
            "-lint".into(),
            "-s".into(),
            path.to_string_lossy().into(),
        ]));
        fs::remove_dir_all(&dir).ok();
    }

    // ── Complex multi-step workflow ─────────────────────────────────

    #[test]
    fn test_workflow_create_populate_convert_extract() {
        let dir = temp_dir();
        let path = dir.join("workflow.plist");

        // 1. Create empty plist
        assert!(run(&[
            "-create".into(),
            "xml1".into(),
            path.to_string_lossy().into()
        ]));

        // 2. Insert values
        assert!(run(&[
            "-insert".into(),
            "app".into(),
            "-string".into(),
            "BurdettieOS".into(),
            path.to_string_lossy().into(),
        ]));
        assert!(run(&[
            "-insert".into(),
            "version".into(),
            "-integer".into(),
            "1".into(),
            path.to_string_lossy().into(),
        ]));
        assert!(run(&[
            "-insert".into(),
            "features".into(),
            "-array".into(),
            path.to_string_lossy().into(),
        ]));
        assert!(run(&[
            "-insert".into(),
            "features.0".into(),
            "-string".into(),
            "macho".into(),
            path.to_string_lossy().into(),
        ]));
        assert!(run(&[
            "-insert".into(),
            "features.1".into(),
            "-string".into(),
            "arm64".into(),
            path.to_string_lossy().into(),
        ]));

        // 3. Lint
        assert!(run(&["-lint".into(), path.to_string_lossy().into()]));

        // 4. Convert to JSON
        let json_path = dir.join("workflow.json");
        assert!(run(&[
            "-convert".into(),
            "json".into(),
            "-r".into(),
            "-o".into(),
            json_path.to_string_lossy().into(),
            path.to_string_lossy().into(),
        ]));
        let j: JsonValue = serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
        assert_eq!(j["app"], "BurdettieOS");
        assert_eq!(j["features"].as_array().unwrap().len(), 2);

        // 5. Replace version
        assert!(run(&[
            "-replace".into(),
            "version".into(),
            "-integer".into(),
            "2".into(),
            path.to_string_lossy().into(),
        ]));

        // 6. Remove a feature
        assert!(run(&[
            "-remove".into(),
            "features.0".into(),
            path.to_string_lossy().into(),
        ]));

        // 7. Extract remaining feature
        let feat_out = dir.join("feat.txt");
        assert!(run(&[
            "-extract".into(),
            "features.0".into(),
            "raw".into(),
            "-o".into(),
            feat_out.to_string_lossy().into(),
            path.to_string_lossy().into(),
        ]));
        assert_eq!(fs::read_to_string(&feat_out).unwrap().trim(), "arm64");

        // 8. Type check
        assert!(run(&[
            "-type".into(),
            "version".into(),
            path.to_string_lossy().into(),
        ]));

        // 9. Convert to binary and back
        let bin_path = dir.join("workflow.bplist");
        assert!(run(&[
            "-convert".into(),
            "binary1".into(),
            "-o".into(),
            bin_path.to_string_lossy().into(),
            path.to_string_lossy().into(),
        ]));
        let final_xml = dir.join("final.plist");
        assert!(run(&[
            "-convert".into(),
            "xml1".into(),
            "-o".into(),
            final_xml.to_string_lossy().into(),
            bin_path.to_string_lossy().into(),
        ]));

        let v = Value::from_file(&final_xml).unwrap();
        let d = v.as_dictionary().unwrap();
        assert_eq!(d.get("app").unwrap().as_string(), Some("BurdettieOS"));
        assert_eq!(d.get("version").unwrap().as_signed_integer(), Some(2));
        assert_eq!(d.get("features").unwrap().as_array().unwrap().len(), 1);

        fs::remove_dir_all(&dir).ok();
    }

    // ── Empty string key ────────────────────────────────────────────

    #[test]
    fn test_extract_empty_keypath_returns_root() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        let out = dir.join("root.json");
        assert!(run(&[
            "-extract".into(),
            "".into(),
            "json".into(),
            "-o".into(),
            out.to_string_lossy().into(),
            path.to_string_lossy().into(),
        ]));
        let j: JsonValue = serde_json::from_str(&fs::read_to_string(&out).unwrap()).unwrap();
        assert!(j.is_object());
        assert_eq!(j["Name"], "BurdettieOS");
        fs::remove_dir_all(&dir).ok();
    }

    // ── Pretty print structure ──────────────────────────────────────

    #[test]
    fn test_pretty_print_runs() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        // Just verify it succeeds (output goes to stdout)
        assert!(run(&["-p".into(), path.to_string_lossy().into()]));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_pretty_print_nonexistent_fails() {
        assert!(!run(&["-p".into(), "/nonexistent.plist".into()]));
    }

    // ── Insert with -json type ──────────────────────────────────────

    #[test]
    fn test_insert_json_value() {
        let dir = temp_dir();
        let path = write_xml_plist(
            &dir,
            "jinsert.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict/></plist>"#,
        );
        assert!(run(&[
            "-insert".into(),
            "config".into(),
            "-json".into(),
            r#"{"debug":true,"level":5}"#.into(),
            path.to_string_lossy().into(),
        ]));
        let v = Value::from_file(&path).unwrap();
        let config = v
            .as_dictionary()
            .unwrap()
            .get("config")
            .unwrap()
            .as_dictionary()
            .unwrap();
        assert_eq!(config.get("debug").unwrap().as_boolean(), Some(true));
        assert_eq!(config.get("level").unwrap().as_signed_integer(), Some(5));
        fs::remove_dir_all(&dir).ok();
    }

    // ── Help ────────────────────────────────────────────────────────

    #[test]
    fn test_help() {
        assert!(run(&["-help".into()]));
    }

    // ── Replace with empty keypath replaces root ────────────────────

    #[test]
    fn test_replace_empty_keypath_replaces_root() {
        let dir = temp_dir();
        let path = write_xml_plist(
            &dir,
            "rootreplace.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict><key>old</key><string>data</string></dict></plist>"#,
        );
        assert!(run(&[
            "-replace".into(),
            "".into(),
            "-string".into(),
            "new root".into(),
            path.to_string_lossy().into(),
        ]));
        let v = Value::from_file(&path).unwrap();
        assert_eq!(v.as_string(), Some("new root"));
        fs::remove_dir_all(&dir).ok();
    }

    // ── Remove from non-container path ──────────────────────────────

    #[test]
    fn test_remove_non_dict_key_from_array_fails() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        // Try to remove "abc" from Tags (an array) — not a valid index
        assert!(!run(&[
            "-remove".into(),
            "Tags.abc".into(),
            path.to_string_lossy().into(),
        ]));
        fs::remove_dir_all(&dir).ok();
    }

    // ── Extract empty string ────────────────────────────────────────

    #[test]
    fn test_extract_empty_string() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        let out = dir.join("empty.txt");
        assert!(run(&[
            "-extract".into(),
            "Empty".into(),
            "raw".into(),
            "-o".into(),
            out.to_string_lossy().into(),
            path.to_string_lossy().into(),
        ]));
        // Should produce just a newline (empty string + newline)
        let content = fs::read_to_string(&out).unwrap();
        assert_eq!(content, "\n");
        fs::remove_dir_all(&dir).ok();
    }

    // ── Extract raw date ────────────────────────────────────────────

    #[test]
    fn test_extract_raw_date() {
        let dir = temp_dir();
        let path = write_xml_plist(
            &dir,
            "date.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>created</key>
    <date>2026-04-07T12:00:00Z</date>
</dict>
</plist>"#,
        );
        let out = dir.join("date.txt");
        assert!(run(&[
            "-extract".into(),
            "created".into(),
            "raw".into(),
            "-o".into(),
            out.to_string_lossy().into(),
            path.to_string_lossy().into(),
        ]));
        assert_eq!(
            fs::read_to_string(&out).unwrap().trim(),
            "2026-04-07T12:00:00Z"
        );
        fs::remove_dir_all(&dir).ok();
    }

    // ── Extract as binary1 ──────────────────────────────────────────

    #[test]
    fn test_extract_as_binary() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        let out = dir.join("settings.bplist");
        assert!(run(&[
            "-extract".into(),
            "Settings".into(),
            "binary1".into(),
            "-o".into(),
            out.to_string_lossy().into(),
            path.to_string_lossy().into(),
        ]));
        let bytes = fs::read(&out).unwrap();
        assert!(bytes.starts_with(b"bplist"));
        let v = Value::from_file(&out).unwrap();
        assert_eq!(
            v.as_dictionary().unwrap().get("Theme").unwrap().as_string(),
            Some("dark")
        );
        fs::remove_dir_all(&dir).ok();
    }

    // ── Format preservation: JSON ───────────────────────────────────

    #[test]
    fn test_insert_preserves_json_format() {
        let dir = temp_dir();
        // Create an XML plist and convert to JSON to get a valid JSON plist file
        let xml_path = write_xml_plist(
            &dir,
            "src.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>existing</key>
    <string>yes</string>
</dict>
</plist>"#,
        );
        let json_path = dir.join("preserve.json");
        assert!(run(&[
            "-convert".into(),
            "json".into(),
            "-o".into(),
            json_path.to_string_lossy().into(),
            xml_path.to_string_lossy().into(),
        ]));

        // Verify it's JSON format
        let bytes = fs::read(&json_path).unwrap();
        assert_eq!(detect_format(&bytes), "json");

        fs::remove_dir_all(&dir).ok();
    }

    // ── Replace preserves binary format ─────────────────────────────

    #[test]
    fn test_replace_preserves_binary_format() {
        let dir = temp_dir();
        let path = dir.join("rp.bplist");
        let mut d = Dictionary::new();
        d.insert("key".to_string(), Value::Integer(1.into()));
        Value::Dictionary(d).to_file_binary(&path).unwrap();

        assert!(run(&[
            "-replace".into(),
            "key".into(),
            "-integer".into(),
            "99".into(),
            path.to_string_lossy().into(),
        ]));

        let bytes = fs::read(&path).unwrap();
        assert!(
            bytes.starts_with(b"bplist"),
            "should preserve binary format after replace"
        );
        let v = Value::from_file(&path).unwrap();
        assert_eq!(
            v.as_dictionary()
                .unwrap()
                .get("key")
                .unwrap()
                .as_signed_integer(),
            Some(99)
        );
        fs::remove_dir_all(&dir).ok();
    }

    // ── Remove preserves binary format ──────────────────────────────

    #[test]
    fn test_remove_preserves_binary_format() {
        let dir = temp_dir();
        let path = dir.join("rm.bplist");
        let mut d = Dictionary::new();
        d.insert("keep".to_string(), Value::Boolean(true));
        d.insert("drop".to_string(), Value::Boolean(false));
        Value::Dictionary(d).to_file_binary(&path).unwrap();

        assert!(run(&[
            "-remove".into(),
            "drop".into(),
            path.to_string_lossy().into(),
        ]));

        let bytes = fs::read(&path).unwrap();
        assert!(
            bytes.starts_with(b"bplist"),
            "should preserve binary format after remove"
        );
        let v = Value::from_file(&path).unwrap();
        let d = v.as_dictionary().unwrap();
        assert!(d.get("keep").is_some());
        assert!(d.get("drop").is_none());
        fs::remove_dir_all(&dir).ok();
    }

    // ── Convert in-place (no -o) ────────────────────────────────────

    #[test]
    fn test_convert_in_place() {
        let dir = temp_dir();
        let path = write_xml_plist(
            &dir,
            "inplace.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>k</key>
    <string>v</string>
</dict>
</plist>"#,
        );
        // Convert to binary in-place
        assert!(run(&[
            "-convert".into(),
            "binary1".into(),
            path.to_string_lossy().into(),
        ]));
        let bytes = fs::read(&path).unwrap();
        assert!(
            bytes.starts_with(b"bplist"),
            "should be binary after in-place convert"
        );

        // Convert back to xml in-place
        assert!(run(&[
            "-convert".into(),
            "xml1".into(),
            path.to_string_lossy().into(),
        ]));
        let content = fs::read_to_string(&path).unwrap();
        assert!(
            content.contains("<?xml"),
            "should be XML after in-place convert back"
        );
        fs::remove_dir_all(&dir).ok();
    }

    // ── Convert binary→json and json→xml ────────────────────────────

    #[test]
    fn test_convert_binary_to_json() {
        let dir = temp_dir();
        let path = dir.join("b2j.bplist");
        let mut d = Dictionary::new();
        d.insert("msg".to_string(), Value::String("hello".into()));
        Value::Dictionary(d).to_file_binary(&path).unwrap();

        let json_path = dir.join("b2j.json");
        assert!(run(&[
            "-convert".into(),
            "json".into(),
            "-o".into(),
            json_path.to_string_lossy().into(),
            path.to_string_lossy().into(),
        ]));
        let j: JsonValue = serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
        assert_eq!(j["msg"], "hello");
        fs::remove_dir_all(&dir).ok();
    }

    // ── Lint binary plist ───────────────────────────────────────────

    #[test]
    fn test_lint_binary_plist() {
        let dir = temp_dir();
        let path = dir.join("lint.bplist");
        let d = Dictionary::new();
        Value::Dictionary(d).to_file_binary(&path).unwrap();
        assert!(run(&["-lint".into(), path.to_string_lossy().into()]));
        fs::remove_dir_all(&dir).ok();
    }

    // ── Print binary plist ──────────────────────────────────────────

    #[test]
    fn test_print_binary_plist() {
        let dir = temp_dir();
        let path = dir.join("print.bplist");
        let mut d = Dictionary::new();
        d.insert("x".to_string(), Value::Integer(42.into()));
        Value::Dictionary(d).to_file_binary(&path).unwrap();
        assert!(run(&["-p".into(), path.to_string_lossy().into()]));
        fs::remove_dir_all(&dir).ok();
    }

    // ── Convert multiple files ──────────────────────────────────────

    #[test]
    fn test_convert_multiple_files() {
        let dir = temp_dir();
        let p1 = write_xml_plist(
            &dir,
            "m1.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict><key>a</key><string>1</string></dict></plist>"#,
        );
        let p2 = write_xml_plist(
            &dir,
            "m2.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict><key>b</key><string>2</string></dict></plist>"#,
        );
        // Convert both to binary in-place
        assert!(run(&[
            "-convert".into(),
            "binary1".into(),
            p1.to_string_lossy().into(),
            p2.to_string_lossy().into(),
        ]));
        assert!(fs::read(&p1).unwrap().starts_with(b"bplist"));
        assert!(fs::read(&p2).unwrap().starts_with(b"bplist"));
        fs::remove_dir_all(&dir).ok();
    }

    // ── Print multiple files ────────────────────────────────────────

    #[test]
    fn test_print_multiple_files() {
        let dir = temp_dir();
        let p1 = write_xml_plist(
            &dir,
            "pr1.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict/></plist>"#,
        );
        let p2 = write_xml_plist(
            &dir,
            "pr2.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict/></plist>"#,
        );
        assert!(run(&[
            "-p".into(),
            p1.to_string_lossy().into(),
            p2.to_string_lossy().into(),
        ]));
        fs::remove_dir_all(&dir).ok();
    }

    // ── Create multiple files ───────────────────────────────────────

    #[test]
    fn test_create_multiple_files() {
        let dir = temp_dir();
        let p1 = dir.join("c1.plist");
        let p2 = dir.join("c2.plist");
        assert!(run(&[
            "-create".into(),
            "xml1".into(),
            p1.to_string_lossy().into(),
            p2.to_string_lossy().into(),
        ]));
        assert!(p1.exists());
        assert!(p2.exists());
        let v1 = Value::from_file(&p1).unwrap();
        let v2 = Value::from_file(&p2).unwrap();
        assert!(v1.as_dictionary().unwrap().is_empty());
        assert!(v2.as_dictionary().unwrap().is_empty());
        fs::remove_dir_all(&dir).ok();
    }

    // ── Insert/replace/remove with -o output path ───────────────────

    #[test]
    fn test_insert_with_output_path() {
        let dir = temp_dir();
        let src = write_rich_plist(&dir);
        let out = dir.join("inserted.plist");
        assert!(run(&[
            "-insert".into(),
            "NewKey".into(),
            "-string".into(),
            "new".into(),
            "-o".into(),
            out.to_string_lossy().into(),
            src.to_string_lossy().into(),
        ]));
        // Original unchanged
        let orig = Value::from_file(&src).unwrap();
        assert!(orig.as_dictionary().unwrap().get("NewKey").is_none());
        // Output has the new key
        let v = Value::from_file(&out).unwrap();
        assert_eq!(
            v.as_dictionary()
                .unwrap()
                .get("NewKey")
                .unwrap()
                .as_string(),
            Some("new")
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_replace_with_output_path() {
        let dir = temp_dir();
        let src = write_rich_plist(&dir);
        let out = dir.join("replaced.plist");
        assert!(run(&[
            "-replace".into(),
            "Name".into(),
            "-string".into(),
            "NewName".into(),
            "-o".into(),
            out.to_string_lossy().into(),
            src.to_string_lossy().into(),
        ]));
        // Original unchanged
        let orig = Value::from_file(&src).unwrap();
        assert_eq!(
            orig.as_dictionary()
                .unwrap()
                .get("Name")
                .unwrap()
                .as_string(),
            Some("BurdettieOS")
        );
        // Output has the replaced value
        let v = Value::from_file(&out).unwrap();
        assert_eq!(
            v.as_dictionary().unwrap().get("Name").unwrap().as_string(),
            Some("NewName")
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_remove_with_output_path() {
        let dir = temp_dir();
        let src = write_rich_plist(&dir);
        let out = dir.join("removed.plist");
        assert!(run(&[
            "-remove".into(),
            "Score".into(),
            "-o".into(),
            out.to_string_lossy().into(),
            src.to_string_lossy().into(),
        ]));
        // Original still has Score
        let orig = Value::from_file(&src).unwrap();
        assert!(orig.as_dictionary().unwrap().get("Score").is_some());
        // Output doesn't
        let v = Value::from_file(&out).unwrap();
        assert!(v.as_dictionary().unwrap().get("Score").is_none());
        fs::remove_dir_all(&dir).ok();
    }

    // ── Insert with empty keypath replaces root ─────────────────────

    #[test]
    fn test_insert_empty_keypath_replaces_root() {
        let dir = temp_dir();
        let path = write_xml_plist(
            &dir,
            "rootins.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict><key>old</key><string>val</string></dict></plist>"#,
        );
        assert!(run(&[
            "-insert".into(),
            "".into(),
            "-array".into(),
            path.to_string_lossy().into(),
        ]));
        let v = Value::from_file(&path).unwrap();
        assert!(v.as_array().unwrap().is_empty());
        fs::remove_dir_all(&dir).ok();
    }

    // ── Unicode and special characters ──────────────────────────────

    #[test]
    fn test_insert_unicode_string() {
        let dir = temp_dir();
        let path = write_xml_plist(
            &dir,
            "uni.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict/></plist>"#,
        );
        assert!(run(&[
            "-insert".into(),
            "emoji".into(),
            "-string".into(),
            "Hello 🌍 café ñ 日本語".into(),
            path.to_string_lossy().into(),
        ]));
        let v = Value::from_file(&path).unwrap();
        assert_eq!(
            v.as_dictionary().unwrap().get("emoji").unwrap().as_string(),
            Some("Hello 🌍 café ñ 日本語")
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_insert_unicode_key() {
        let dir = temp_dir();
        let path = write_xml_plist(
            &dir,
            "unikey.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict/></plist>"#,
        );
        assert!(run(&[
            "-insert".into(),
            "名前".into(),
            "-string".into(),
            "テスト".into(),
            path.to_string_lossy().into(),
        ]));
        let v = Value::from_file(&path).unwrap();
        assert_eq!(
            v.as_dictionary().unwrap().get("名前").unwrap().as_string(),
            Some("テスト")
        );
        fs::remove_dir_all(&dir).ok();
    }

    // ── No-newline option for extract ───────────────────────────────

    #[test]
    fn test_extract_raw_no_newline() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        let out = dir.join("nonl.txt");
        assert!(run(&[
            "-extract".into(),
            "Name".into(),
            "raw".into(),
            "-n".into(),
            "-o".into(),
            out.to_string_lossy().into(),
            path.to_string_lossy().into(),
        ]));
        let content = fs::read_to_string(&out).unwrap();
        // No trailing newline
        assert_eq!(content, "BurdettieOS");
        fs::remove_dir_all(&dir).ok();
    }

    // ── No-newline option for JSON convert ──────────────────────────

    #[test]
    fn test_convert_json_no_newline() {
        let dir = temp_dir();
        let path = write_xml_plist(
            &dir,
            "nonl.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>x</key>
    <integer>1</integer>
</dict>
</plist>"#,
        );
        let json_path = dir.join("nonl.json");
        assert!(run(&[
            "-convert".into(),
            "json".into(),
            "-n".into(),
            "-o".into(),
            json_path.to_string_lossy().into(),
            path.to_string_lossy().into(),
        ]));
        let content = fs::read_to_string(&json_path).unwrap();
        assert!(!content.ends_with('\n'), "should not end with newline");
        fs::remove_dir_all(&dir).ok();
    }

    // ── plist_to_json edge cases ────────────────────────────────────

    #[test]
    fn test_plist_to_json_empty_dict() {
        let v = Value::Dictionary(Dictionary::new());
        let j = plist_to_json(&v);
        assert!(j.is_object());
        assert_eq!(j.as_object().unwrap().len(), 0);
    }

    #[test]
    fn test_plist_to_json_empty_array() {
        let v = Value::Array(vec![]);
        let j = plist_to_json(&v);
        assert!(j.is_array());
        assert_eq!(j.as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_plist_to_json_real_infinity() {
        // Infinity can't be represented in JSON → Null
        let j = plist_to_json(&Value::Real(f64::INFINITY));
        assert!(j.is_null());
        let j = plist_to_json(&Value::Real(f64::NEG_INFINITY));
        assert!(j.is_null());
    }

    #[test]
    fn test_plist_to_json_empty_data() {
        let j = plist_to_json(&Value::Data(vec![]));
        assert_eq!(j.as_str(), Some(""));
    }

    #[test]
    fn test_plist_to_json_empty_string() {
        let j = plist_to_json(&Value::String(String::new()));
        assert_eq!(j.as_str(), Some(""));
    }

    #[test]
    fn test_plist_to_json_bool_false() {
        let j = plist_to_json(&Value::Boolean(false));
        assert_eq!(j, false);
    }

    #[test]
    fn test_plist_to_json_integer_negative() {
        let j = plist_to_json(&Value::Integer((-100).into()));
        assert_eq!(j.as_i64(), Some(-100));
    }

    // ── json_to_plist edge cases ────────────────────────────────────

    #[test]
    fn test_json_to_plist_bool_true() {
        let v = json_to_plist(&serde_json::json!(true));
        assert_eq!(v.as_boolean(), Some(true));
    }

    #[test]
    fn test_json_to_plist_bool_false() {
        let v = json_to_plist(&serde_json::json!(false));
        assert_eq!(v.as_boolean(), Some(false));
    }

    #[test]
    fn test_json_to_plist_integer() {
        let v = json_to_plist(&serde_json::json!(42));
        assert_eq!(v.as_signed_integer(), Some(42));
    }

    #[test]
    fn test_json_to_plist_negative_integer() {
        let v = json_to_plist(&serde_json::json!(-99));
        assert_eq!(v.as_signed_integer(), Some(-99));
    }

    #[test]
    fn test_json_to_plist_string() {
        let v = json_to_plist(&serde_json::json!("hello"));
        assert_eq!(v.as_string(), Some("hello"));
    }

    #[test]
    fn test_json_to_plist_empty_array() {
        let v = json_to_plist(&serde_json::json!([]));
        assert!(v.as_array().unwrap().is_empty());
    }

    #[test]
    fn test_json_to_plist_empty_object() {
        let v = json_to_plist(&serde_json::json!({}));
        assert!(v.as_dictionary().unwrap().is_empty());
    }

    // ── Large plist stress test ─────────────────────────────────────

    #[test]
    fn test_large_plist_many_keys() {
        let dir = temp_dir();
        let path = dir.join("large.plist");
        assert!(run(&[
            "-create".into(),
            "xml1".into(),
            path.to_string_lossy().into()
        ]));
        // Insert 100 keys
        for i in 0..100 {
            assert!(
                run(&[
                    "-insert".into(),
                    format!("key_{i}"),
                    "-integer".into(),
                    format!("{i}"),
                    path.to_string_lossy().into(),
                ]),
                "failed to insert key_{i}"
            );
        }
        // Lint
        assert!(run(&["-lint".into(), path.to_string_lossy().into()]));
        // Verify count
        let v = Value::from_file(&path).unwrap();
        assert_eq!(v.as_dictionary().unwrap().len(), 100);
        // Extract a few spot checks
        assert_eq!(
            v.as_dictionary()
                .unwrap()
                .get("key_0")
                .unwrap()
                .as_signed_integer(),
            Some(0)
        );
        assert_eq!(
            v.as_dictionary()
                .unwrap()
                .get("key_99")
                .unwrap()
                .as_signed_integer(),
            Some(99)
        );
        fs::remove_dir_all(&dir).ok();
    }

    // ── Large array ─────────────────────────────────────────────────

    #[test]
    fn test_large_array() {
        let dir = temp_dir();
        let path = dir.join("arr_large.plist");
        assert!(run(&[
            "-create".into(),
            "xml1".into(),
            path.to_string_lossy().into()
        ]));
        assert!(run(&[
            "-insert".into(),
            "items".into(),
            "-array".into(),
            path.to_string_lossy().into(),
        ]));
        for i in 0..50 {
            assert!(run(&[
                "-insert".into(),
                format!("items.{i}"),
                "-integer".into(),
                format!("{i}"),
                path.to_string_lossy().into(),
            ]));
        }
        let v = Value::from_file(&path).unwrap();
        let items = v
            .as_dictionary()
            .unwrap()
            .get("items")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(items.len(), 50);
        assert_eq!(items[0].as_signed_integer(), Some(0));
        assert_eq!(items[49].as_signed_integer(), Some(49));
        fs::remove_dir_all(&dir).ok();
    }

    // ── Insert date type via CLI ────────────────────────────────────

    #[test]
    fn test_insert_date() {
        let dir = temp_dir();
        let path = write_xml_plist(
            &dir,
            "insdate.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict/></plist>"#,
        );
        assert!(run(&[
            "-insert".into(),
            "created".into(),
            "-date".into(),
            "2026-04-07T12:00:00Z".into(),
            path.to_string_lossy().into(),
        ]));
        let v = Value::from_file(&path).unwrap();
        assert!(v
            .as_dictionary()
            .unwrap()
            .get("created")
            .unwrap()
            .as_date()
            .is_some());
        fs::remove_dir_all(&dir).ok();
    }

    // ── Insert XML type via CLI ─────────────────────────────────────

    #[test]
    fn test_insert_xml_type() {
        let dir = temp_dir();
        let path = write_xml_plist(
            &dir,
            "insxml.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict/></plist>"#,
        );
        let xml_val = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><array><integer>1</integer><integer>2</integer></array></plist>"#;
        assert!(run(&[
            "-insert".into(),
            "nested".into(),
            "-xml".into(),
            xml_val.into(),
            path.to_string_lossy().into(),
        ]));
        let v = Value::from_file(&path).unwrap();
        let arr = v
            .as_dictionary()
            .unwrap()
            .get("nested")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(arr.len(), 2);
        fs::remove_dir_all(&dir).ok();
    }

    // ── Type multiple files ─────────────────────────────────────────

    #[test]
    fn test_type_multiple_files() {
        let dir = temp_dir();
        let p1 = write_xml_plist(
            &dir,
            "t1.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict><key>a</key><string>s</string></dict></plist>"#,
        );
        let p2 = write_xml_plist(
            &dir,
            "t2.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict><key>a</key><integer>1</integer></dict></plist>"#,
        );
        assert!(run(&[
            "-type".into(),
            "a".into(),
            p1.to_string_lossy().into(),
            p2.to_string_lossy().into(),
        ]));
        fs::remove_dir_all(&dir).ok();
    }

    // ── Default lint with invalid file ──────────────────────────────

    #[test]
    fn test_default_lint_invalid() {
        let dir = temp_dir();
        let path = write_xml_plist(&dir, "bad.plist", "not a plist {{{");
        assert!(!run(&[path.to_string_lossy().into()]));
        fs::remove_dir_all(&dir).ok();
    }

    // ── Extract with -n no-newline for JSON ─────────────────────────

    #[test]
    fn test_extract_json_no_newline() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        let out = dir.join("nonljson.json");
        assert!(run(&[
            "-extract".into(),
            "Settings".into(),
            "json".into(),
            "-n".into(),
            "-o".into(),
            out.to_string_lossy().into(),
            path.to_string_lossy().into(),
        ]));
        let content = fs::read_to_string(&out).unwrap();
        assert!(!content.ends_with('\n'), "should not end with newline");
        let j: JsonValue = serde_json::from_str(&content).unwrap();
        assert_eq!(j["Theme"], "dark");
        fs::remove_dir_all(&dir).ok();
    }

    // ── parse_type_and_value with option-like next arg ──────────────

    #[test]
    fn test_parse_type_and_value_bool_followed_by_option() {
        // -bool followed by -s (an option flag) → no value captured
        let args: Vec<String> = vec!["-bool".into(), "-s".into(), "file.plist".into()];
        let (t, v, rest) = parse_type_and_value(&args);
        assert_eq!(t.as_deref(), Some("-bool"));
        assert!(v.is_none());
        assert_eq!(rest, vec!["-s", "file.plist"]);
    }

    #[test]
    fn test_parse_type_and_value_string_only() {
        // -string with no value and no more args
        let args: Vec<String> = vec!["-string".into()];
        let (t, v, rest) = parse_type_and_value(&args);
        assert_eq!(t.as_deref(), Some("-string"));
        assert!(v.is_none());
        assert!(rest.is_empty());
    }

    #[test]
    fn test_parse_type_and_value_data_with_value() {
        let args: Vec<String> = vec!["-data".into(), "SGVsbG8=".into(), "file.plist".into()];
        let (t, v, rest) = parse_type_and_value(&args);
        assert_eq!(t.as_deref(), Some("-data"));
        assert_eq!(v.as_deref(), Some("SGVsbG8="));
        assert_eq!(rest, vec!["file.plist"]);
    }

    #[test]
    fn test_parse_type_and_value_date_with_value() {
        let args: Vec<String> = vec!["-date".into(), "2026-04-07T00:00:00Z".into()];
        let (t, v, rest) = parse_type_and_value(&args);
        assert_eq!(t.as_deref(), Some("-date"));
        assert_eq!(v.as_deref(), Some("2026-04-07T00:00:00Z"));
        assert!(rest.is_empty());
    }

    // ── get_output_path with both -o and -e (precedence) ────────────

    #[test]
    fn test_get_output_path_o_takes_precedence_over_e() {
        let opts = Options {
            output_path: Some("/tmp/explicit.plist".into()),
            extension: Some("json".into()),
            ..Options::default()
        };
        // -o should win over -e
        assert_eq!(get_output_path("input.plist", &opts), "/tmp/explicit.plist");
    }

    // ── Insert/replace on nonexistent file ──────────────────────────

    #[test]
    fn test_insert_nonexistent_file() {
        assert!(!run(&[
            "-insert".into(),
            "key".into(),
            "-string".into(),
            "val".into(),
            "/nonexistent/dir/file.plist".into(),
        ]));
    }

    #[test]
    fn test_replace_nonexistent_file() {
        assert!(!run(&[
            "-replace".into(),
            "key".into(),
            "-string".into(),
            "val".into(),
            "/nonexistent/dir/file.plist".into(),
        ]));
    }

    #[test]
    fn test_remove_nonexistent_file() {
        assert!(!run(&[
            "-remove".into(),
            "key".into(),
            "/nonexistent/dir/file.plist".into(),
        ]));
    }

    #[test]
    fn test_extract_nonexistent_file() {
        assert!(!run(&[
            "-extract".into(),
            "key".into(),
            "raw".into(),
            "/nonexistent/dir/file.plist".into(),
        ]));
    }

    #[test]
    fn test_type_nonexistent_file() {
        assert!(!run(&[
            "-type".into(),
            "key".into(),
            "/nonexistent/dir/file.plist".into(),
        ]));
    }

    #[test]
    fn test_convert_nonexistent_file() {
        assert!(!run(&[
            "-convert".into(),
            "json".into(),
            "/nonexistent/dir/file.plist".into(),
        ]));
    }

    // ── Lint silent invalid ─────────────────────────────────────────

    #[test]
    fn test_lint_silent_invalid() {
        let dir = temp_dir();
        let path = write_xml_plist(&dir, "silbad.plist", "not valid {{{");
        assert!(!run(&[
            "-lint".into(),
            "-s".into(),
            path.to_string_lossy().into(),
        ]));
        fs::remove_dir_all(&dir).ok();
    }

    // ── Replace nested value ────────────────────────────────────────

    #[test]
    fn test_replace_nested_dict_value() {
        let dir = temp_dir();
        let path = write_rich_plist(&dir);
        assert!(run(&[
            "-replace".into(),
            "Settings.Theme".into(),
            "-string".into(),
            "light".into(),
            path.to_string_lossy().into(),
        ]));
        let v = Value::from_file(&path).unwrap();
        assert_eq!(
            v.as_dictionary()
                .unwrap()
                .get("Settings")
                .unwrap()
                .as_dictionary()
                .unwrap()
                .get("Theme")
                .unwrap()
                .as_string(),
            Some("light")
        );
        fs::remove_dir_all(&dir).ok();
    }

    // ── Insert/replace with -bool false ─────────────────────────────

    #[test]
    fn test_insert_bool_false() {
        let dir = temp_dir();
        let path = write_xml_plist(
            &dir,
            "boolfalse.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict/></plist>"#,
        );
        assert!(run(&[
            "-insert".into(),
            "disabled".into(),
            "-bool".into(),
            "NO".into(),
            path.to_string_lossy().into(),
        ]));
        let v = Value::from_file(&path).unwrap();
        assert_eq!(
            v.as_dictionary()
                .unwrap()
                .get("disabled")
                .unwrap()
                .as_boolean(),
            Some(false)
        );
        fs::remove_dir_all(&dir).ok();
    }

    // ── Detect format for edge cases ────────────────────────────────

    #[test]
    fn test_detect_format_whitespace_before_xml() {
        // Leading whitespace before XML declaration
        assert_eq!(detect_format(b"  <?xml..."), "xml1");
    }

    #[test]
    fn test_detect_format_json_array() {
        assert_eq!(detect_format(b"[1,2,3]"), "json");
    }

    #[test]
    fn test_detect_format_single_byte() {
        assert_eq!(detect_format(b"x"), "xml1");
    }

    // ── Insert multiple values sequentially verifies order ──────────

    #[test]
    fn test_insert_order_preserved() {
        let dir = temp_dir();
        let path = write_xml_plist(
            &dir,
            "order.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict/></plist>"#,
        );
        for key in &["cherry", "apple", "banana"] {
            assert!(run(&[
                "-insert".into(),
                (*key).into(),
                "-string".into(),
                (*key).into(),
                path.to_string_lossy().into(),
            ]));
        }
        let v = Value::from_file(&path).unwrap();
        let d = v.as_dictionary().unwrap();
        assert_eq!(d.len(), 3);
        assert_eq!(d.get("cherry").unwrap().as_string(), Some("cherry"));
        assert_eq!(d.get("apple").unwrap().as_string(), Some("apple"));
        assert_eq!(d.get("banana").unwrap().as_string(), Some("banana"));
        fs::remove_dir_all(&dir).ok();
    }

    // ── Create with -o output path ──────────────────────────────────

    #[test]
    fn test_create_with_output_path() {
        let dir = temp_dir();
        let dummy = dir.join("src.plist"); // needed as "file" arg
        let out = dir.join("created_out.plist");
        assert!(run(&[
            "-create".into(),
            "xml1".into(),
            "-o".into(),
            out.to_string_lossy().into(),
            dummy.to_string_lossy().into(),
        ]));
        assert!(out.exists());
        let v = Value::from_file(&out).unwrap();
        assert!(v.as_dictionary().unwrap().is_empty());
        fs::remove_dir_all(&dir).ok();
    }
}
