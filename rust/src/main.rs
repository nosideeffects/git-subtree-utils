use serde::Deserialize;
use std::process::{Command, exit};
use std::path::Path;
use std::io::BufReader;
use std::fs::File;
use clap::{App, SubCommand, Arg};
use dialoguer::Input;

fn main() {
    let subtree_arg = Arg::with_name("SUBTREE")
        .help("Sets the subtree to use")
        .required(true)
        .index(1);
    let matches = App::new("gitstu")
        .version("0.0.1")
        .about("Helper utility for working with git subtrees")
        .author("Jacob Biggs <biggs.jacob@gmail.com>")
        .subcommand(SubCommand::with_name("init")
            .about("Creates a .gitstu for this repository"))
        .subcommand(SubCommand::with_name("add")
            .about("Define a new subtree configuration")
            .arg(subtree_arg.clone()))
        .subcommand(SubCommand::with_name("pull")
            .about("Pulls a subtree from a remote")
            .arg(subtree_arg.clone()))
        .subcommand(SubCommand::with_name("push")
            .about("Pushes a subtree to a remote")
            .arg(subtree_arg.clone()))
        .subcommand(SubCommand::with_name("refresh")
            .about("Retrieves remote branch information"))
        .get_matches();

    let git_root = get_git_root();
    println!("Git root: {}", git_root);

    let config = load_config(&git_root);
    println!("{:?}", config);

    if let (subcommand, Some(args)) = matches.subcommand() {
        match subcommand {
            "pull"|"push"|"add" => {
                let subtree_name = args.value_of("SUBTREE").unwrap();

                println!("{:?}: {:?}", subcommand, subtree_name);

                match config.subtrees.iter().find(|s| s.name == subtree_name) {
                    Some(subtree_config) => {
                        match subcommand {
                            "pull" => {pull_subtree(subtree_config)}
                            "push" => {}
                            "add" => {add_subtree(subtree_config)}
                            _ => {panic!()}
                        }

                    },
                    None => {
                        eprintln!("Subtree {:?} not found in .gitstu", subtree_name);
                        eprintln!("To define a new subtree: gitstu add {}", subtree_name);
                    }
                }
            }
            "init" => {}
            _ => panic!("Unrecognized subcommand")
        }
    }
}

fn load_config(git_root: &String) -> GitStuConfig {
    let path = Path::join(git_root.as_ref(), ".gitstu");
    println!("{:?}", path);
    let file = File::open(path).expect("Unable to find .gitstu file");
    let reader = BufReader::new(file);
    let config = serde_json::from_reader(reader).expect("Unable to parse .gitstu");

    config
}

fn pull_subtree(subtree_config: &SubtreeConfig) {
    let (branch, remote) = branch_and_remote(subtree_config);

    println!("Pulling branch {:?} from remote {:?}", branch, remote);
    Command::new("sh")
        .arg("-c")
        .arg(format!("git subtree pull --prefix={} {} {}", subtree_config.prefix, remote, branch))
        .spawn()
        .expect("Failed to pull subtree");
}

fn add_subtree(subtree_config: &SubtreeConfig) {
    let (branch, remote) = branch_and_remote(subtree_config);

    println!("Add branch {:?} from remote {:?}", branch, remote);
    Command::new("sh")
        .arg("-c")
        .arg(format!("git subtree add --prefix={} {} {}", subtree_config.prefix, remote, branch))
        .spawn()
        .expect("Failed to add subtree");
}

fn branch_and_remote(subtree_config: &SubtreeConfig) -> (String, String) {
    let branch = subtree_config.branch.clone().unwrap_or_else(|| {
        Input::new().with_prompt("Branch name")
            .default("master".to_string())
            .interact()
            .unwrap()
    });
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

#[derive(Deserialize, Debug)]
struct GitStuConfig {
    subtrees: Vec<SubtreeConfig>
}

#[derive(Deserialize, Debug)]
struct SubtreeConfig {
    name: String,
    prefix: String,
    branch: Option<String>,
    remote: Option<String>
}
