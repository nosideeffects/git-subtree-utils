use serde::{Deserialize, Serialize};
use std::process::{Command, exit};
use std::path::{Path, PathBuf};
use std::io::{BufReader, BufWriter};
use std::fs::File;
use clap::{App, SubCommand, Arg, ArgMatches};
use dialoguer::{Input, Confirmation};
use itertools::Itertools;

fn main() {
    let subtree_arg = Arg::with_name("SUBTREE")
        .help("Sets the subtree to use")
        .required_unless_one(&["all", "with-branch"])
        .index(1);
    let branch_arg = Arg::with_name("BRANCH")
        .help("Sets which branch to use")
        .required(false)
        .conflicts_with_all(&["branch", "to-branch"])
        .index(2);
    let to_branch_arg = Arg::with_name("to-branch")
        .help("Sets the branch to push to")
        .short("t")
        .long("to-branch")
        .takes_value(true);
    let matches = App::new("gitstu")
        .version("0.0.1")
        .about("Helper utility for working with git subtrees")
        .author("Jacob Biggs <biggs.jacob@gmail.com>")
        .arg(Arg::with_name("remote")
            .help("Sets the remote to use")
            .short("r")
            .long("remote")
            .takes_value(true)
            .global(true))
        .arg(Arg::with_name("prefix")
            .help("Sets the prefix to use")
            .short("p")
            .long("prefix")
            .takes_value(true)
            .global(true))
        .arg(Arg::with_name("branch")
            .help("Sets the branch to use")
            .short("b")
            .long("branch")
            .takes_value(true)
            .global(true))
        .arg(Arg::with_name("squash")
            .help("Squashes commits")
            .short("s")
            .long("squash")
            .global(true))
        .arg(Arg::with_name("all")
            .help("Runs command against all subtrees")
            .short("a")
            .long("all")
            .conflicts_with("SUBTREE")
            .global(true))
        .arg(Arg::with_name("with-branch")
            .help("Filter currently to subtree with current branch")
            .short("w")
            .long("with-branch")
            .takes_value(true)
            .requires("all")
            .global(true))
        .subcommand(SubCommand::with_name("init")
            .about("Creates a .gitstu for this repository"))
        .subcommand(SubCommand::with_name("add")
            .about("Define a new subtree configuration")
            .arg(&subtree_arg)
            .arg(&branch_arg))
        .subcommand(SubCommand::with_name("pull")
            .about("Pulls a subtree from a remote")
            .arg(&subtree_arg)
            .arg(&branch_arg)
            .arg(&to_branch_arg))
        .subcommand(SubCommand::with_name("push")
            .about("Pushes a subtree to a remote")
            .arg(&subtree_arg)
            .arg(&branch_arg)
            .arg(&to_branch_arg))
        .subcommand(SubCommand::with_name("refresh")
            .about("Retrieves remote branch information"))
        .get_matches();

    let git_root = get_git_root();
    let config_path = Path::join(git_root.as_ref(), ".gitstu");

    if let (subcommand, Some(args)) = matches.subcommand() {
        match subcommand {
            "pull"|"push"|"add" => {
                let mut config = load_config(&config_path);
                let mode = config.mode.clone().unwrap_or(SubtreeMode::SUBTREE);
                let branch_arg = args.value_of("BRANCH").or(args.value_of("branch"));
                let squash = args.is_present("squash") || config.squash.unwrap_or(false);
                let all_subtrees = args.is_present("all");

                let subtrees = if all_subtrees {
                    if let Some(with_branch) = args.value_of("with-branch") {
                        let subtrees = config.subtrees.iter().cloned()
                            .filter(|s| s.branch == Some(with_branch.to_string())).collect();
                        Some(subtrees)
                    } else {
                        Some(config.subtrees.clone())
                    }
                } else {
                    let subtree_name = args.value_of("SUBTREE").unwrap();
                    match config.subtrees.iter_mut().find(|s| s.name == subtree_name) {
                        Some(subtree_config) => {
                            let subtrees = vec![subtree_config.clone()];
                            Some(subtrees)
                        },
                        None => {
                            match subcommand {
                                "add" => {
                                    let remote_arg = args.value_of("remote");
                                    let prefix_arg = args.value_of("prefix");
                                    let subtree_name = subtree_name.to_string();
                                    let remote_url = remote_arg.map(Into::into).or_else(||
                                        Some(prompt_for("remote url", Some("".to_string()))))
                                        .unwrap();
                                    let remote_alias = remote_arg.map(Into::into).or_else(||
                                        Some(prompt_for("remote alias", Some(subtree_name.clone()))))
                                        .unwrap();
                                    let subtree_config = SubtreeConfig {
                                        name: subtree_name.clone(),
                                        prefix: prefix_arg.map(Into::into).unwrap_or_else(|| prompt_for("prefix", Some(subtree_name))),
                                        branch: branch_arg.map(Into::into).or_else(||
                                            Some(prompt_for("branch", Some("master".to_string())))),
                                        remote: Some(GitRemote {
                                            url: remote_url,
                                            alias: remote_alias
                                        })
                                    };

                                    let subtrees = vec![subtree_config.clone()];
                                    config.subtrees.push(subtree_config);
                                    Some(subtrees.clone())
                                }
                                _ => {
                                    eprintln!("Subtree {:?} not found in .gitstu", subtree_name);
                                    eprintln!("To define a new subtree: gitstu add {}", subtree_name);
                                    None
                                }
                            }
                        }
                    }
                };

                println!("{:?}", subtrees);
                if let Some(mut subtrees) = subtrees {
                    match mode {
                        SubtreeMode::SUBTREE => {
                            for mut subtree_config in &mut subtrees {
                                match subcommand {
                                    "pull" => { pull_subtree(&mut subtree_config, &args, squash) }
                                    "push" => { push_subtree(&mut subtree_config, &args) }
                                    "add" => { add_subtree(&mut subtree_config, branch_arg, squash) }
                                    _ => { panic!() }
                                }
                            }

                            subtrees.append(&mut config.subtrees);
                        }
                        SubtreeMode::CUSTOM => {
                            for mut subtree_config in &mut subtrees {
                                match subcommand {
                                    "pull" => { merge_subtree(&mut subtree_config, &args, squash) }
                                    // TODO: Need branch, cherry-pick, and merge
                                    "push" => { push_subtree(&mut subtree_config, &args) }
                                    "add" => { read_tree(&mut subtree_config, branch_arg) }
                                    _ => { panic!() }
                                }
                            }

                            subtrees.append(&mut config.subtrees);

                        }
                    }
                    config.subtrees = subtrees.into_iter().unique_by(|subtree| subtree.name.clone()).collect();
                }

                save_config(&config_path, config);
            }
            "init" => {unimplemented!()}
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
    serde_json::to_writer_pretty(writer, &config).expect("Failed to save .gitstu");
}

fn prompt_for(name: &str, default: Option<String>) -> String {
    let mut prompt = Input::new();
    prompt.with_prompt(name)
        .allow_empty(false);

    if let Some(default) = default {
        prompt.default(default);
    }

    prompt.interact().unwrap()
}

fn pull_subtree(subtree_config: &mut SubtreeConfig, args: &ArgMatches, squash: bool) {
    let branch_arg = args.value_of("BRANCH")
        .or(args.value_of("to-branch"))
        .or(args.value_of("branch"));
    let (branch, remote) = branch_and_remote(subtree_config, branch_arg);

    println!("Pulling branch {:?} from remote {:?}", branch, remote.alias);
    let mut command = format!("git subtree pull --prefix={} {} {}", subtree_config.prefix, remote.alias, branch);
    if squash {
        command.push_str(" --squash");
    }
    Command::new("sh")
        .arg("-c")
        .arg(command)
        .spawn()
        .expect("Failed to pull subtree")
        .wait()
        .expect("Failed to pull subtree");

    persist_branch_name(subtree_config, &branch);
    persist_remote(subtree_config, &remote);
}

fn push_subtree(subtree_config: &mut SubtreeConfig, args: &ArgMatches) {
    let branch_arg = args.value_of("BRANCH")
        .or(args.value_of("to-branch"))
        .or(args.value_of("branch"));
    let (branch, remote) = branch_and_remote(subtree_config, branch_arg);

    println!("Pushing branch {:?} to remote {:?}", branch, remote.alias);
    Command::new("sh")
        .arg("-c")
        .arg(format!("git subtree push --prefix={} {} {}", subtree_config.prefix, remote.alias, branch))
        .spawn()
        .expect("Failed to push subtree")
        .wait()
        .expect("Failed to push subtree");

    if !args.is_present("all") && !args.is_present("to-branch") {
        persist_branch_name(subtree_config, &branch);
        persist_remote(subtree_config, &remote);
    }
}

fn add_subtree(subtree_config: &mut SubtreeConfig, branch_arg: Option<&str>, squash: bool) {
    let (branch, remote) = branch_and_remote(subtree_config, branch_arg);

    println!("Add branch {:?} from remote {:?}", branch, remote.alias);
    let mut command = format!("git subtree add --prefix={} {} {}", subtree_config.prefix, remote.alias, branch);
    if squash {
        command.push_str(" --squash");
    }
    Command::new("sh")
        .arg("-c")
        .arg(command)
        .spawn()
        .expect("Failed to add subtree")
        .wait()
        .expect("Failed to add subtree");

    persist_branch_name(subtree_config, &branch);
    persist_remote(subtree_config, &remote);
}

fn merge_subtree(subtree_config: &mut SubtreeConfig, args: &ArgMatches, squash: bool) {
    let branch_arg = args.value_of("BRANCH")
        .or(args.value_of("to-branch"))
        .or(args.value_of("branch"));
    let (branch, remote) = branch_and_remote(subtree_config, branch_arg);

    println!("Pulling branch {:?} from remote {:?}", branch, remote.alias);
    let mut command = format!("git merge -X subtree={}/ --strategy-options-theirs {}/{}", subtree_config.prefix, remote.alias, branch);
    if squash {
        command.push_str(" --squash");
    }
    Command::new("sh")
        .arg("-c")
        .arg(command)
        .spawn()
        .expect("Failed to merge subtree")
        .wait()
        .expect("Failed to merge subtree");

    persist_branch_name(subtree_config, &branch);
    persist_remote(subtree_config, &remote);
}

/// Use `git read-tree` command to pull in current state of a subtree for the first time, ignoring
/// all of its historical commits.
fn read_tree(subtree_config: &mut SubtreeConfig, branch_arg: Option<&str>) {
    let (branch, remote) = branch_and_remote(subtree_config, branch_arg);

    println!("Add branch {:?} from remote {:?}", branch, remote.alias);
    let command = format!("git read-tree --prefix={} {}/{}", subtree_config.prefix, remote.alias, branch);
    Command::new("sh")
        .arg("-c")
        .arg(command)
        .spawn()
        .expect("Failed to add subtree")
        .wait()
        .expect("Failed to add subtree");

    persist_branch_name(subtree_config, &branch);
    persist_remote(subtree_config, &remote);
}

/// Prompts the user to persist provided branch name to their .gitstu config if
/// it differs from the name currently persisted or if there is none persisted
fn persist_branch_name(subtree_config: &mut SubtreeConfig, branch: &String) {
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

/// Prompts the user to persist provided remote to their .gitstu config if
/// it differs from the remote currently persisted or if there is none persisted
fn persist_remote(subtree_config: &mut SubtreeConfig, remote: &GitRemote) {
    let remote_to_persist = match &subtree_config.remote {
        Some(remote_name) => {
            if remote_name != remote {
                Some(remote)
            } else {
                None
            }
        }
        None => Some(remote)
    };
    if let Some(remote) = remote_to_persist {
        let confirmation = Confirmation::new()
            .with_text(format!("Do you want to save remote {:?} ({}) to .gitstu?", remote.alias, remote.url).as_ref())
            .interact();

        match confirmation {
            Ok(_) => {}
            _ => { println!("Unable to read user input, not persisting remote") }
        }
        subtree_config.branch = Some(remote.alias.to_string());
    }
}

fn branch_and_remote(subtree_config: &SubtreeConfig, branch_arg: Option<&str>) -> (String, GitRemote) {
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
        let remote_url = Input::new()
            .with_prompt("Git remote url")
            .interact().unwrap();
        let remote_alias = Input::new()
            .with_prompt("Git remote alias")
            .default(subtree_config.name.clone())
            .interact().unwrap();
        GitRemote {
            url: remote_url,
            alias: remote_alias
        }
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
    #[serde(skip_serializing_if = "Option::is_none")]
    squash: Option<bool>,
    mode: Option<SubtreeMode>,
    subtrees: Vec<SubtreeConfig>
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct SubtreeConfig {
    name: String,
    prefix: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    remote: Option<GitRemote>
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct GitRemote {
    url: String,
    alias: String
}

impl PartialEq for GitRemote {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url && self.alias == other.alias
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
enum SubtreeMode {
    #[serde(rename = "subtree")]
    SUBTREE,
    #[serde(rename = "custom")]
    CUSTOM
}
