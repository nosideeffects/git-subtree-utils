use serde::{Deserialize, Serialize};
use std::process::{Command, exit};
use std::path::{Path, PathBuf};
use std::io::{BufReader, BufWriter};
use std::fs::File;
use clap::{App, SubCommand, Arg};
use dialoguer::{Input, Confirmation};

fn main() {
    let subtree_arg = Arg::with_name("SUBTREE")
        .help("Sets the subtree to use")
        .required(true)
        .index(1);
    let branch_arg = Arg::with_name("BRANCH")
        .help("Selects which branch to use")
        .required(false)
        .index(2);
    let matches = App::new("gitstu")
        .version("0.0.1")
        .about("Helper utility for working with git subtrees")
        .author("Jacob Biggs <biggs.jacob@gmail.com>")
        .subcommand(SubCommand::with_name("init")
            .about("Creates a .gitstu for this repository"))
        .subcommand(SubCommand::with_name("add")
            .about("Define a new subtree configuration")
            .arg(&subtree_arg)
            .arg(&branch_arg))
        .subcommand(SubCommand::with_name("pull")
            .about("Pulls a subtree from a remote")
            .arg(&subtree_arg)
            .arg(&branch_arg))
        .subcommand(SubCommand::with_name("push")
            .about("Pushes a subtree to a remote")
            .arg(&subtree_arg)
            .arg(&branch_arg))
        .subcommand(SubCommand::with_name("refresh")
            .about("Retrieves remote branch information"))
        .get_matches();

    let git_root = get_git_root();
    let config_path = Path::join(git_root.as_ref(), ".gitstu");

    if let (subcommand, Some(args)) = matches.subcommand() {
        match subcommand {
            "pull"|"push"|"add" => {
                let mut config = load_config(&config_path);
                let subtree_name = args.value_of("SUBTREE").unwrap();
                let branch_name = args.value_of("BRANCH");

                println!("{:?}: {:?}", subcommand, subtree_name);

                match config.subtrees.iter_mut().find(|s| s.name == subtree_name) {
                    Some(subtree_config) => {
                        match subcommand {
                            "pull" => {pull_subtree(subtree_config, branch_name)}
                            "push" => {push_subtree(subtree_config, branch_name)}
                            "add" => {add_subtree(subtree_config, branch_name)}
                            _ => {panic!()}
                        }

                    },
                    None => {
                        eprintln!("Subtree {:?} not found in .gitstu", subtree_name);
                        eprintln!("To define a new subtree: gitstu add {}", subtree_name);
                    }
                }

                save_config(&config_path, config);
            }
            "init" => {}
            _ => panic!("Unrecognized subcommand")
        }
    }
}

fn load_config(path: &PathBuf) -> GitStuConfig {
    println!("{:?}", path);
    let file = File::open(path).expect("Unable to find .gitstu file");
    let reader = BufReader::new(file);
    let config = serde_json::from_reader(reader).expect("Unable to parse .gitstu");

    config
}

fn save_config(path: &PathBuf, mut config: GitStuConfig) {
    let file = File::create(path).expect("Unable to create .gitsu file");
    let writer = BufWriter::new(file);
    config.subtrees.sort_by(|a, b| a.name.cmp(&b.name));
    serde_json::to_writer_pretty(writer, &config);
}

fn pull_subtree(subtree_config: &mut SubtreeConfig, branch_arg: Option<&str>) {
    let (branch, remote) = branch_and_remote(subtree_config, branch_arg);

    println!("Pulling branch {:?} from remote {:?}", branch, remote);
    Command::new("sh")
        .arg("-c")
        .arg(format!("git subtree pull --prefix={} {} {}", subtree_config.prefix, remote, branch))
        .spawn()
        .expect("Failed to pull subtree")
        .wait();

    persist_branch_name(subtree_config, &branch);
}

fn push_subtree(subtree_config: &mut SubtreeConfig, branch_arg: Option<&str>) {
    let (branch, remote) = branch_and_remote(subtree_config, branch_arg);

    println!("Pushing branch {:?} to remote {:?}", branch, remote);
    Command::new("sh")
        .arg("-c")
        .arg(format!("git subtree push --prefix={} {} {}", subtree_config.prefix, remote, branch))
        .spawn()
        .expect("Failed to pull subtree")
        .wait();

    persist_branch_name(subtree_config, &branch);
}

/// Prompts the user to persist provided branch name to their .gitstu config if
/// it differs from the name currently persisted or if there is none persisted
fn persist_branch_name(subtree_config: &mut SubtreeConfig, branch: &String) -> () {
    let branch_to_persist = match &subtree_config.branch {
        Some(branch_name) => {
            if branch_name != branch {
                Some(branch)
            } else {
                None
            }
        },
        None => Some(branch)
    };
    if let Some(branch_name) = branch_to_persist {
        let confirmation = Confirmation::new()
            .with_text(format!("Do you want to save branch {:?} to .gitstu?", branch_name).as_ref())
            .interact();

        match confirmation {
            Ok(_) => {}
            _ => { println!("Unable to read user input, not persisting branch") }
        }
        subtree_config.branch = Some(branch_name.to_string());
    }
}

fn add_subtree(subtree_config: &mut SubtreeConfig, branch_arg: Option<&str>) {
    let (branch, remote) = branch_and_remote(subtree_config, branch_arg);

    println!("Add branch {:?} from remote {:?}", branch, remote);
    Command::new("sh")
        .arg("-c")
        .arg(format!("git subtree add --prefix={} {} {}", subtree_config.prefix, remote, branch))
        .spawn()
        .expect("Failed to add subtree")
        .wait();

    persist_branch_name(subtree_config, &branch);
}

fn branch_and_remote(subtree_config: &SubtreeConfig, branch_arg: Option<&str>) -> (String, String) {
    let branch = {
        if let Some(provided_branch) = branch_arg {
            provided_branch.to_string()
        } else {
            subtree_config.branch.clone().unwrap_or_else(|| {
                let default_branch = branch_arg.unwrap_or("master");
                Input::new().with_prompt("Branch name")
                    .default(default_branch.to_string())
                    .interact()
                    .unwrap()
            })
        }
    };
    let remote = subtree_config.remote.clone().unwrap_or_else(|| {
        Input::new().with_prompt("Git remote or url").interact().unwrap()
    });
    (branch, remote)
}

fn get_git_root() -> String {
    let output = Command::new("sh")
        .arg("-c")
        .arg("git rev-parse --show-toplevel")
        .output()
        .expect("Failed to retrieve git root");

    match output.status.code() {
        Some(0) => {
            std::str::from_utf8(&output.stdout).unwrap().trim().to_string()
        },
        _ => {
            eprintln!("Unable to locate git root!\nEnsure you are within a git repository and try again...");
            exit(1);
        }
    }
}

#[derive(Deserialize, Debug, Serialize)]
struct GitStuConfig {
    subtrees: Vec<SubtreeConfig>
}

#[derive(Deserialize, Debug, Serialize)]
struct SubtreeConfig {
    name: String,
    prefix: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    remote: Option<String>
}
