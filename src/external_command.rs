use crate::git::commit_object::CommitObject;
use std::env;
use std::io::Write;
use std::process::{Command, Stdio};

/// Check if `git` command exists in your environment
pub fn check() -> Result<std::process::Output, std::io::Error> {
    Command::new("git").output()
}

/// Check if there are unstaged changes
pub fn check_unstaged() -> bool {
    String::from_utf8(
        Command::new("git")
            .args(&["status", "-s"])
            .output()
            .expect("Error in checking git status")
            .stdout,
    )
    .expect("Error in getting output")
    .is_empty()
}

/// Get current working directory path string
pub fn current_dir_path() -> String {
    env::current_dir().unwrap().to_str().unwrap().to_owned()
}

/// Get latest commit hash string by using `git log`
pub fn latest_commit_hash(path: &str) -> String {
    String::from_utf8(
        Command::new("git")
            .args(vec!["-C", &path, "log", "-1", "--format=%H"])
            .output()
            .unwrap_or_else(|_| {
                panic!(format!(
                    "Error in executing `git -C {} log` -1 --format=%H",
                    path
                ))
            })
            .stdout,
    )
    .expect("Error in getting output")
    .trim_end()
    .to_owned()
}

/// Get commit object file content using `git cat-file -p <hash>`
pub fn cat_file(path: &str, hash: &str) -> String {
    String::from_utf8(
        Command::new("git")
            .args(vec!["-C", &path, "cat-file", "-p", hash])
            .output()
            .unwrap_or_else(|_| {
                panic!(format!(
                    "Erorr in executing `git -C {} cat-file -p {}`",
                    path, hash
                ))
            })
            .stdout,
    )
    .expect("Error in getting output")
}

/// Replace the latest commit's committer name using git commit-tree + git reset --soft.
/// Returns the new commit hash.
pub fn replace_latest_commit(path: &str, co: &CommitObject, new_committer_name: &str) -> String {
    let author_email = format!("{}@{}", co.author.email_user, co.author.email_domain);
    let committer_email = format!("{}@{}", co.committer.email_user, co.committer.email_domain);

    let mut args = vec![
        "-C".to_owned(), path.to_owned(),
        "commit-tree".to_owned(), co.tree.clone(),
    ];
    if let Some(ref parent) = co.parent {
        args.push("-p".to_owned());
        args.push(parent.clone());
    }
    args.extend_from_slice(&["-F".to_owned(), "-".to_owned()]);

    let mut child = Command::new("git")
        .args(&args)
        .env("GIT_AUTHOR_NAME", &co.author.name)
        .env("GIT_AUTHOR_EMAIL", &author_email)
        .env("GIT_AUTHOR_DATE", &co.author.time)
        .env("GIT_COMMITTER_NAME", new_committer_name)
        .env("GIT_COMMITTER_EMAIL", &committer_email)
        .env("GIT_COMMITTER_DATE", &co.committer.time)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Error spawning git commit-tree");

    child
        .stdin
        .take()
        .unwrap()
        .write_all(co.message.as_bytes())
        .expect("Error writing commit message to git commit-tree");

    let output = child.wait_with_output().expect("Error waiting for git commit-tree");
    let new_hash = String::from_utf8(output.stdout)
        .expect("Error reading hash from git commit-tree")
        .trim_end()
        .to_owned();

    Command::new("git")
        .args(&["-C", path, "reset", "--soft", &new_hash])
        .output()
        .expect("Error in git reset --soft");

    new_hash
}
