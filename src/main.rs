use anyhow::Result;
use clap::{App, SubCommand};
use dialoguer::{theme::ColorfulTheme, Select};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::prelude::*;
use std::io::BufReader;
use std::process;
use std::process::Command;
#[derive(Clone)]
struct DialogItem {
    name: &'static str,
    nested: Option<Dialog>,
    command: Option<&'static str>,
    description: String,
}

#[derive(Clone)]
struct Dialog {
    prompt: &'static str,
    items: Vec<DialogItem>,
}

fn get_release_versions() -> Result<(String, String, String)> {
    let mut reader = BufReader::new(fs::File::open("./.buildconfig-android.yml")?);
    let mut s = String::new();
    reader.read_line(&mut s)?;
    let deserialized_map: BTreeMap<String, String> = serde_yaml::from_str(&s)?;
    let version = deserialized_map
        .get(&"libraryVersion".to_string())
        .ok_or_else(|| anyhow::Error::msg("No version available"))?
        .clone();
    let mut versions_split = version.split_terminator(".");
    let major = versions_split
        .next()
        .ok_or_else(|| anyhow::Error::msg("No major version"))?;
    let minor = versions_split
        .next()
        .ok_or_else(|| anyhow::Error::msg("No minor version"))?;
    let patch = versions_split
        .next()
        .ok_or_else(|| anyhow::Error::msg("No patch version"))?;
    let new_major_num = (major.parse::<u32>()? + 1).to_string();
    let new_minor_num = (minor.parse::<u32>()? + 1).to_string();
    let new_patch_num = (patch.parse::<u32>()? + 1).to_string();
    Ok((
        new_major_num
            .chars()
            .chain(".0.0".chars())
            .collect::<String>(),
        major
            .chars()
            .chain(std::iter::once('.'))
            .chain(new_minor_num.chars())
            .chain(".0".chars())
            .collect::<String>(),
        major
            .chars()
            .chain(std::iter::once('.'))
            .chain(minor.chars())
            .chain(std::iter::once('.'))
            .chain(new_patch_num.chars())
            .collect::<String>(),
    ))
}

fn setup_default_dialog() -> Dialog {
    let (major_release, minor_release, patch_release) =
        get_release_versions().unwrap_or(("".to_string(), "".to_string(), "".to_string()));
    let commands = vec![
        DialogItem {
            name: "build",
            nested: None,
            command: Some("cargo build"),
            description: "Build Application Services".to_string(),
        },
        DialogItem {
            name: "test",
            nested: None,
            command: Some("sh ./automation/all_tests.sh"),
            description: "Run all tests for a pull request".to_string(),
        },
        DialogItem {
            name: "test_rust",
            command: Some("sh ./automation/all_rust_tests.sh"),
            nested: None,
            description: "Run all Rust tests".to_string(),
        },
        DialogItem {
            name: "verify_env",
            description: "Verify your development environment".to_string(),
            command: Some(
                "sh ./libs/verify-desktop-environment.sh ;
        sh ./libs/verify-android-environment.sh ;
        sh ./libs/verify-ios-environment.sh",
            ),
            nested: None,
        },
        DialogItem {
            name: "release",
            description: "Prepare a release".to_string(),
            nested: Some(Dialog {
                prompt: "What type of release would like to prepare?",
                items: vec![
                    DialogItem {
                        name: "release-major",
                        description: format!("Prepare a major release {}", major_release),
                        nested: None,
                        command: Some("python3 ./automation/prepare-release.py major"),
                    },
                    DialogItem {
                        name: "release-minor",
                        description: format!("Prepare a minor release {}", minor_release),
                        nested: None,
                        command: Some("python3 ./automation/prepare-release.py minor"),
                    },
                    DialogItem {
                        name: "release-patch",
                        description: format!("Prepare a patch release {}", patch_release),
                        nested: None,
                        command: Some("python3 ./automation/prepare-release.py patch"),
                    },
                ],
            }),
            command: None,
        },
        DialogItem {
            name: "regen-dependencies",
            nested: None,
            command: Some("sh ./tools/regenerate_dependency_summaries.sh"),
            description: "Regenerate dependency summaries".to_string(),
        },
        DialogItem {
            name: "lint_bash",
            nested: None,
            command: Some("sh ./automation/lint_bash_scripts.sh"),
            description: "Lint bash script changes".to_string(),
        },
        DialogItem {
            name: "cargo_update",
            description: "Create a 'cargo update' PR".to_string(),
            command: Some("python3 ./automation/cargo-update-pr.py"),
            nested: None,
        },
        DialogItem {
            name: "regen-protobufs",
            description: "Regenerate protobuf files".to_string(),
            command: Some("cargo run --bin protobuf-gen tools/protobuf_files.toml"),
            nested: None,
        },
        DialogItem {
            name: "help",
            nested: None,
            command: Some("cargo asdev -h"),
            description: "See all CLI options".to_string(),
        },
    ];

    Dialog {
        prompt: "What would you like to do today?",
        items: commands.clone(),
    }
}

fn main() {
    let default_dialog = setup_default_dialog();

    let empty_dialog = Dialog {
        prompt: "",
        items: vec![],
    };
    // Flatten up the nested dialogs
    // For power users to be able to call directly
    // Currently only two levels exists, so that's what we flatten
    let flattened_iter = default_dialog.items.iter().chain(
        default_dialog
            .items
            .iter()
            .flat_map(|d| d.nested.as_ref().unwrap_or(&empty_dialog).items.iter()),
    );
    let map: HashMap<String, DialogItem> = flattened_iter
        .clone()
        .map(|dialog| (dialog.name.to_string(), dialog.clone()))
        .collect();
    let matches =
        App::new(env!("CARGO_PKG_NAME"))
            .bin_name("cargo")
            .subcommand(
                SubCommand::with_name("asdev")
                    .about("Helps you navigate the Application Services repository")
                    .author("Application Services Team")
                    .version(env!("CARGO_PKG_VERSION"))
                    .subcommands(flattened_iter.map(|dialog| {
                        SubCommand::with_name(dialog.name).about(&*dialog.description)
                    })),
            )
            .get_matches();

    if let Some(matches) = matches.subcommand_matches("asdev") {
        match map.get(matches.subcommand().0) {
            Some(val) => run_dialog_item(val),
            None => run_dialog(&default_dialog),
        }
    } else {
        run_dialog(&default_dialog)
    }
}

struct CommandOutput {
    command: String,
    status: process::ExitStatus,
}

fn run_dialog(dialog: &Dialog) {
    let selections = dialog
        .items
        .iter()
        .map(|dialog_item| dialog_item.description.clone())
        .collect::<Vec<_>>();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(dialog.prompt)
        .default(0)
        .items(&selections[..])
        .interact()
        .unwrap();

    let cmd = &dialog.items[selection];
    run_dialog_item(cmd);
}

fn run_dialog_item(dialog_item: &DialogItem) {
    if let Some(dialog) = &dialog_item.nested {
        run_dialog(dialog)
    } else {
        spawn(dialog_item.command.unwrap())
    }
}

fn spawn(cmd: &str) {
    let commands: Vec<&str> = cmd.split_terminator(';').collect();
    let mut threads = Vec::new();
    commands.iter().for_each(|cmd| {
        let cmd = cmd.to_string();
        threads.push(std::thread::spawn(move || {
            let mut split = cmd.split_whitespace();

            let output = Command::new(split.next().unwrap())
                .args(split)
                .stdout(process::Stdio::inherit())
                .stderr(process::Stdio::inherit())
                .spawn()
                .unwrap()
                .wait_with_output()
                .unwrap();
            CommandOutput {
                command: cmd.to_string(),
                status: output.status,
            }
        }));
    });
    let mut outputs = Vec::new();
    for thread in threads {
        outputs.push(thread.join().unwrap());
    }
    println!("\n==========  Report  ==========\n");
    outputs.iter().for_each(|command_output| {
        if command_output.status.success() {
            println!("{} ✅\n", command_output.command.trim());
        } else {
            println!("{} ❌\n", command_output.command.trim());
        }
    });
    if let Some(output) = outputs.iter().find(|output| !output.status.success()) {
        std::process::exit(output.status.code().unwrap());
    }
}
