// Copyright 2022 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::process::{exit, Command};
use std::{env, fs};

const FEAT_OPTIONS: [bool; 2] = [false, true];
const FEAT_BYTEMUCK: &str = "bytemuck";
const FEAT_SERDE: &str = "serde";
const FEAT_STD: &str = "std";

fn run_cmd(mut cmd: Command) {
    println!("Running: {}", format!("{:?}", cmd).replace('"', ""));
    let status = cmd.status().expect("failed to launch");
    if !status.success() {
        panic!("command failed: {}", status);
    }
}

#[derive(Clone, Copy)]
enum CargoAction {
    Test,
    Lint,
}

impl CargoAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Lint => "clippy",
            Self::Test => "test",
        }
    }
}

fn get_cargo_cmd(
    action: CargoAction,
    package: &str,
    features: &[&str],
) -> Command {
    let mut cmd = Command::new("cargo");
    cmd.args([action.as_str(), "--package", package]);
    if !features.is_empty() {
        cmd.args(["--features", &features.join(",")]);
    }
    match action {
        CargoAction::Test => {}
        CargoAction::Lint => {
            cmd.args(["--", "-D", "warnings"]);
        }
    }
    cmd
}

fn test_package(package: &str, features: &[&str]) {
    run_cmd(get_cargo_cmd(CargoAction::Lint, package, features));
    run_cmd(get_cargo_cmd(CargoAction::Test, package, features));
}

fn test_uguid() {
    for feat_bytemuck in FEAT_OPTIONS {
        for feat_serde in FEAT_OPTIONS {
            for feat_std in FEAT_OPTIONS {
                let mut features = Vec::new();
                if feat_bytemuck {
                    features.push(FEAT_BYTEMUCK);
                }
                if feat_serde {
                    features.push(FEAT_SERDE);
                }
                if feat_std {
                    features.push(FEAT_STD);
                }

                test_package("uguid", &features);
            }
        }
    }
}

fn test_gpt_disk_types() {
    for feat_bytemuck in FEAT_OPTIONS {
        for feat_std in FEAT_OPTIONS {
            let mut features = Vec::new();
            if feat_bytemuck {
                features.push(FEAT_BYTEMUCK);
            }
            if feat_std {
                features.push(FEAT_STD);
            }

            test_package("gpt_disk_types", &features);
        }
    }
}

fn test_gpt_disk_io() {
    for feat_std in FEAT_OPTIONS {
        let mut features = Vec::new();
        if feat_std {
            features.push("std");
        }

        test_package("gpt_disk_types", &features);
    }
}

fn gen_guids() {
    let template = fs::read_to_string("uguid/src/guid_template.rs").unwrap();

    struct Replacements {
        name: &'static str,
        repr: &'static str,
        other: &'static str,
        doc: &'static str,
    }

    let aligned_replacements = Replacements {
        name: "AlignedGuid",
        repr: "C, align(8)",
        other: "Guid",
        doc: r#""Globally-unique identifier (8-byte aligned).

The format is described in Appendix A of the UEFI
Specification. Note that the first three fields are little-endian.

This type is compatible with the `EFI_GUID` type, which is specified
to be 8-byte aligned.""#,
    };

    let unaligned_replacements = Replacements {
        name: "Guid",
        repr: "C",
        other: "AlignedGuid",
        doc: r#""Globally-unique identifier (1-byte aligned).

The format is described in Appendix A of the UEFI
Specification. Note that the first three fields are little-endian.""#,
    };

    let gen_code = |r: Replacements| {
        format!(
            "// This file is autogenerated, do not edit.\n\n{}",
            template
                .replace("VAR_STRUCT_NAME", r.name)
                .replace("VAR_STRUCT_REPR", r.repr)
                .replace("VAR_OTHER_STRUCT_NAME", r.other)
                .replace("VAR_STRUCT_DOC", r.doc)
        )
    };

    let aligned_code = gen_code(aligned_replacements);
    let unaligned_code = gen_code(unaligned_replacements);

    let aligned_path = "uguid/src/aligned_guid.rs";
    let unaligned_path = "uguid/src/unaligned_guid.rs";

    // Check if the generated contents have changed.
    let changed = fs::read_to_string(aligned_path).unwrap() != aligned_code
        || fs::read_to_string(unaligned_path).unwrap() != unaligned_code;

    fs::write("uguid/src/aligned_guid.rs", aligned_code).unwrap();
    fs::write("uguid/src/unaligned_guid.rs", unaligned_code).unwrap();

    // Exit non-zero if contents have changed. This is used in CI to
    // make sure the files are up to date.
    if changed {
        exit(1);
    }
}

fn main() {
    let args: Vec<_> = env::args().collect();
    let arg_test_all = "test_all";
    let arg_test_uguid = "test_uguid";
    let arg_test_gpt_disk_types = "test_gpt_disk_types";
    let arg_test_gpt_disk_io = "test_gpt_disk_io";
    let arg_gen_guids = "gen_guids";
    let actions = &[
        arg_test_all,
        arg_test_uguid,
        arg_test_gpt_disk_types,
        arg_test_gpt_disk_io,
        arg_gen_guids,
    ];
    if args.len() != 2 || !actions.contains(&args[1].as_ref()) {
        println!("usage: cargo xtask [{}]", actions.join("|"));
        exit(1);
    }

    let action = &args[1];
    if action == arg_test_all || action == arg_test_uguid {
        test_uguid();
    }
    if action == arg_test_all || action == arg_test_gpt_disk_types {
        test_gpt_disk_types();
    }
    if action == arg_test_all || action == arg_test_gpt_disk_io {
        test_gpt_disk_io();
    }
    if action == arg_gen_guids {
        gen_guids();
    }
}