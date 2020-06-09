use clap::{App, SubCommand};
use dialoguer::{theme::ColorfulTheme, Select};
use std::collections::HashMap;
use std::process;
use std::process::Command;

const COMMANDS: &[(&str, (&str, &str))] = &[
    ("build", ("Build Application Services", "cargo build")),
    (
        "test",
        (
            "Run all tests for a pull request",
            "sh ./automation/all_tests.sh",
        ),
    ),
    (
        "test_rust",
        ("Run all Rust tests", "sh ./automation/all_rust_tests.sh"),
    ),
    (
        "verify_desktop",
        (
            "Verify Desktop Environment",
            "sh ./libs/verify-desktop-environment.sh",
        ),
    ),
    (
        "verify_android",
        (
            "Verify Android Environment",
            "sh ./libs/verify-android-environment.sh",
        ),
    ),
    (
        "verify_ios",
        (
            "Verify iOS Environment",
            "sh ./libs/verify-ios-environment.sh",
        ),
    ),
    (
        "smoke_android",
        (
            "Smoke test Android Components",
            "python3 ./automation/smoke-test-android-components.py",
        ),
    ),
    (
        "smoke_fenix",
        (
            "Smoke test Fenix Components",
            "python3 ./automation/smoke-test-fenix.py",
        ),
    ),
    (
        "smoke_ios",
        (
            "Smoke test Firefox iOS Components",
            "python3 ./automation/smoke-test-firefox-ios.py",
        ),
    ),
    (
        "tag_minor",
        (
            "Tag a new release minor release",
            "python3 ./automation/prepare-release.py minor",
        ),
    ),
    (
        "prepare_release",
        (
            "Tag a new release patch release",
            "python3 ./automation/prepare-release.py patch",
        ),
    ),
    (
        "lint_bash",
        (
            "Lint bash script changes",
            "sh ./automation/lint_bash_scripts.sh",
        ),
    ),
    (
        "cargo_update",
        (
            "Create a 'cargo update' PR",
            "python3 ./automation/cargo-update-pr.py",
        ),
    ),
];

fn main() {
    let map: HashMap<&str, (&str, &str)> = COMMANDS.iter().cloned().collect();
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .bin_name("cargo")
        .subcommand(
            SubCommand::with_name("asdev")
                .about("Helps you navigate the Application Services repository")
                .author("Application Services Team")
                .version(env!("CARGO_PKG_VERSION"))
                .subcommands(COMMANDS.iter().map(|(key_word, (title, _cmd))| {
                    SubCommand::with_name(key_word).about(*title)
                })),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("asdev") {
        match map.get(matches.subcommand().0) {
            Some(val) => spawn(val.1),
            None => run_default(),
        }
    } else {
        run_default()
    }
}

fn run_default() {
    let selections = COMMANDS
        .iter()
        .map(|(_key_word, (title, _cmd))| title)
        .collect::<Vec<_>>();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("What would you like to do today?")
        .default(0)
        .items(&selections[..])
        .interact()
        .unwrap();

    let cmd = COMMANDS[selection].1;
    spawn(cmd.1);
}

fn spawn(cmd: &str) {
    println!("Executing: {}", cmd);
    let mut split = cmd.split_whitespace();

    let output = Command::new(split.next().unwrap())
        .args(split)
        .stdout(process::Stdio::inherit())
        .stderr(process::Stdio::inherit())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    if let Some(code) = output.status.code() {
        std::process::exit(code);
    } else if !output.status.success() {
        std::process::exit(-1);
    }
}
