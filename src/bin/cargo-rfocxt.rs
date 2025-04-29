use std::{
    env,
    ffi::OsString,
    path::Path,
    process::{exit, Command},
};

use log::error;
use rfocxt::utils::compile_time_sysroot;
use simplelog::{ConfigBuilder, TermLogger};
use time::UtcOffset;

const CARGO_RFOCXT_HELP: &str = r#"Generate focal context for rust program

Usage:
    cargo rfocxt
"#;

fn show_help() {
    println!("{}", CARGO_RFOCXT_HELP);
}

fn has_arg_flag(name: &str) -> bool {
    let mut args = env::args().take_while(|val| val != "--");
    args.any(|val| val == name)
}

fn get_arg_flag_value(name: &str) -> Option<String> {
    let mut args = env::args().take_while(|val| val != "--");
    loop {
        let arg = match args.next() {
            Some(arg) => arg,
            None => return None,
        };
        if !arg.starts_with(name) {
            continue;
        }
        let suffix = &arg[name.len()..];
        if suffix.is_empty() {
            return args.next();
        } else if suffix.starts_with("=") {
            return Some(suffix[1..].to_string());
        }
    }
}

fn current_crate() -> cargo_metadata::Package {
    let manifest_path =
        get_arg_flag_value("--manifest-path").map(|m| Path::new(&m).canonicalize().unwrap());

    let mut cmd = cargo_metadata::MetadataCommand::new();
    if let Some(ref manifest_path) = manifest_path {
        cmd.manifest_path(manifest_path);
    }

    let mut metadata = if let Ok(metadata) = cmd.exec() {
        metadata
    } else {
        error!("Could not obtain Cargo metadata; likely an ill-formed manifest");
        exit(1);
    };

    let current_dir = env::current_dir();

    let package_index = metadata
        .packages
        .iter()
        .position(|package| {
            let package_manifest_path = Path::new(&package.manifest_path);
            if let Some(ref manifest_path) = manifest_path {
                package_manifest_path == manifest_path
            } else {
                let current_dir = current_dir
                    .as_ref()
                    .expect("Could not read current directory");
                let package_manifest_directort = package_manifest_path
                    .parent()
                    .expect("Could not find parent directory of package manifest");
                package_manifest_directort == current_dir
            }
        })
        .unwrap_or_else(|| {
            error!("This seems to be a workspace, which is not supported by cargo-rfocxt ");
            exit(1);
        });

    let package = metadata.packages.remove(package_index);

    package
}

fn rfocxt() -> Command {
    let mut path = env::current_exe().expect("Current executable path invalid");
    path.set_file_name("rfocxt");
    Command::new(path)
}

fn cargo() -> Command {
    Command::new(env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo")))
}

fn in_cargo_rfocxt() {
    let verbose = has_arg_flag("-v");

    let current_crate = current_crate();
    let path = Path::new(&current_crate.manifest_path).parent().unwrap();
    let path_str = path.to_str().unwrap();

    for target in current_crate.targets.into_iter() {
        let mut args = env::args().skip(2);
        let kind = target
            .kind
            .get(0)
            .expect("Badly formatted cargo meradata: target::kind is an empty array")
            .to_string();
        let mut cmd = cargo();
        cmd.arg("check");
        match kind.as_str() {
            "bin" => {
                cmd.arg("--bin").arg(target.name);
            }
            "lib" => {
                cmd.arg("--lib");
            }
            _ => continue,
        }
        while let Some(arg) = args.next() {
            if arg == "--" {
                break;
            }
            cmd.arg(arg);
        }

        let args_vec: Vec<String> = args.collect();
        cmd.env(
            "RFOCXT_ARGS",
            serde_json::to_string(&args_vec).expect("Failed to serialize args"),
        );
        cmd.env("RFOCXT_TOP_CRATE_NAME", current_crate.name.clone());

        let path = env::current_exe().expect("Current executable path invalid");
        cmd.env("RUSTC_WRAPPER", path);

        if verbose {
            cmd.env("RFOCXT_VERBOSE", "");
            eprintln!("+ {:?}", cmd);
        }

        cmd.env("RFOCXT_CRATE_DIR", path_str);

        let exit_status = cmd
            .spawn()
            .expect("Could not run cargo")
            .wait()
            .expect("Failed to wait for cargo");

        if !exit_status.success() {
            exit(exit_status.code().unwrap_or(-1));
        }
    }
}

fn inside_cargo_rustc() {
    let mut cmd = rfocxt();
    cmd.args(env::args().skip(2));

    let sysroot = compile_time_sysroot().expect("Connot find sysroot");
    cmd.arg("--sysroot");
    cmd.arg(sysroot);

    let top_crate_name = env::var("RFOCXT_TOP_CRATE_NAME").expect("Missing RFOCXT_TOP_CRATE_NAME");
    let top_crate_name = top_crate_name.replace("-", "_");

    if get_arg_flag_value("--crate-name").as_deref() == Some(&top_crate_name) {
        let magic = env::var("RFOCXT_ARGS").expect("Missing RFOCXT_ARGS");
        let rfocxt_args: Vec<String> =
            serde_json::from_str(&magic).expect("Failed to deserialize RFOCXT_ARGS");
        cmd.args(rfocxt_args);
    } else {
        cmd.env("RFOCXT_BE_RUSTC", "1");
    }

    let verbose = env::var_os("RFOCXT_VERBOSE").is_some();
    if verbose {
        eprintln!("+ {:#?}", cmd);
    }

    match cmd.status() {
        Ok(exit) => {
            if !exit.success() {
                std::process::exit(exit.code().unwrap_or(-2));
            }
        }
        Err(ref e) => {
            panic!("Error during rfocxt run: {:?}", e);
        }
    }
}

fn main() {
    let time_offset = UtcOffset::from_hms(8, 0, 0).unwrap();
    let log_config = ConfigBuilder::new()
        .set_location_level(log::LevelFilter::Error)
        .set_time_offset(time_offset)
        .build();
    TermLogger::init(
        log::LevelFilter::Info,
        log_config,
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .unwrap();

    if env::args().any(|a| a == "--help" || a == "-h") {
        show_help();
        return;
    }

    if let Some("rfocxt") = env::args().nth(1).as_ref().map(AsRef::as_ref) {
        in_cargo_rfocxt();
    } else if env::args()
        .nth(1)
        .as_ref()
        .map(AsRef::as_ref)
        .is_some_and(|s: &str| s.contains("rustc"))
    {
        inside_cargo_rustc();
    } else {
        error!("cargo-rfocxt must be called with either 'cargo-rfocxt' or `/home/abezbm/.rustup/toolchains/nightly-2024-07-21-x86_64-unknown-linux-gnu/bin/rustc` as first argument.")
    }
}
