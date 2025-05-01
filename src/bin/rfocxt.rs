#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_session;

use rfocxt::analysis::callbacks::RfocxtCallbacks;
use rfocxt::utils::compile_time_sysroot;
use rustc_errors::emitter::HumanReadableErrorType;
use rustc_errors::ColorConfig;
use rustc_session::config::ErrorOutputType;
use rustc_session::EarlyDiagCtxt;
use simplelog::{ConfigBuilder, TermLogger};
use std::path::PathBuf;
use std::{env, process::exit};
use time::UtcOffset;

fn main() {
    let result = rustc_driver::catch_fatal_errors(move || {
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

        let mut rustc_args = env::args_os()
            .enumerate()
            .map(|(i, arg)| arg.into_string().unwrap())
            .collect::<Vec<_>>();

        if let Some(sysroot) = compile_time_sysroot() {
            let sysroot_flag = "--sysroot";
            if !rustc_args.iter().any(|e| e == sysroot_flag) {
                rustc_args.push(sysroot_flag.to_string());
                rustc_args.push(sysroot);
            }
        }

        if env::var_os("RFOCXT_BE_RUSTC").is_some() {
            let early_diag_ctxt = EarlyDiagCtxt::new(ErrorOutputType::HumanReadable(
                HumanReadableErrorType::Default(ColorConfig::Auto),
            ));
            rustc_driver::init_rustc_env_logger(&early_diag_ctxt);

            let mut callbacks = rustc_driver::TimePassesCallbacks::default();
            let run_compiler = rustc_driver::RunCompiler::new(&rustc_args, &mut callbacks);
            let _ = run_compiler.run();
        } else {
            let always_encode_mir = "-Zalways_encode_mir";
            if !rustc_args.iter().any(|e| e == always_encode_mir) {
                rustc_args.push(always_encode_mir.to_string());
            }
            rustc_args.push("-Cpanic=abort".to_string());

            let env = env::var_os("RFOCXT_CRATE_DIR").unwrap();
            let crate_dir = PathBuf::from(env);

            let mut callbacks = RfocxtCallbacks::new(crate_dir);
            let run_compiler = rustc_driver::RunCompiler::new(&rustc_args, &mut callbacks);
            let _ = run_compiler.run();
        }
    });

    let exit_code = match result {
        Ok(_) => 0,
        Err(_) => 1,
    };
    exit(exit_code);
}
