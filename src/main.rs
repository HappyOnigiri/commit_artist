mod external_command;
mod git;
mod settings;

use crate::external_command as command;
use crate::git::commit_object::CommitObject;
use crate::settings::Settings;
use seahorse::{App, Context, Flag, FlagType};
use std::env;
use std::sync::mpsc::channel;
use std::thread;

fn main() {
    let args: Vec<String> = env::args().collect();
    let app = App::new("Commit Artist")
        .author(env!("CARGO_PKG_AUTHORS"))
        .description(env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
        .usage("commit_artist <flags>")
        .flag(
            Flag::new("path", FlagType::String)
                .usage("[optional] --path <path_to_your_repository>"),
        )
        .flag(
            Flag::new("pattern", FlagType::String)
                .usage("[optional] --pattern <[0-9a-f]{1,40}>")
                .alias("p"),
        )
        .flag(
            Flag::new("block", FlagType::Int)
                .usage("[optional] --block 28")
                .alias("b"),
        )
        .flag(
            Flag::new("jobs", FlagType::Int)
                .usage("[optional] --jobs 4")
                .alias("j"),
        )
        .flag(
            Flag::new("force", FlagType::Bool)
                .usage("[optional] --force / -f  Skip unstaged changes check")
                .alias("f"),
        )
        .flag(
            Flag::new("bench", FlagType::Bool)
                .usage("[optional] --bench  Measure single-threaded hash rate and exit"),
        )
        .action(art);

    app.run(args);
}

/// as you see
fn art(c: &Context) {
    let mut settings = Settings::default();

    if let Ok(path) = c.string_flag("path") {
        settings.path = path;
    }

    if let Ok(pattern) = c.string_flag("pattern") {
        settings.patterns(pattern);
    }

    if let Ok(block) = c.int_flag("block") {
        settings.block_size(block as usize);
    }

    if let Ok(jobs) = c.int_flag("jobs") {
        settings.jobs(jobs as usize);
    }

    let force = c.bool_flag("force");
    let bench_mode = c.bool_flag("bench");

    if command::check().is_err() {
        println!("git command not found");
        return;
    }

    if !force && !bench_mode && !command::check_unstaged() {
        println!(
            "There are unstages changes. You should stash or discard them before running this."
        );
        return;
    }

    let latest_commit_hash = command::latest_commit_hash(&settings.path);
    if latest_commit_hash.is_empty() {
        println!("No Commits are Detected.");
        return;
    }

    let latest_cat_file: String = command::cat_file(&settings.path, &latest_commit_hash);
    let co = CommitObject::parse_cat_file(&latest_cat_file);

    if bench_mode {
        bench_hash_rate(&co, settings.jobs);
        return;
    }

    let new_committer_name = bruteforce(settings.clone(), &co, settings.jobs);
    command::filter_branch(&settings.path, &latest_commit_hash, &new_committer_name);
    let latest_commit_hash = command::latest_commit_hash(&settings.path);
    println!(
        "Yay! Now your new hash of the latest commit is \x1b[31m{}\x1b[m.",
        latest_commit_hash
    );
}

/// Measure hash rate across `jobs` threads and report aggregate hashes/sec.
fn bench_hash_rate(commit_object: &CommitObject, jobs: usize) {
    use std::time::{Duration, Instant};

    let duration = Duration::from_secs(5);
    let start = Instant::now(); // Instant は Copy なのでスレッドにそのまま渡せる
    let (tx, rx) = channel::<u64>();

    for i in 0..jobs {
        let mut co = commit_object.clone();
        let tx = tx.clone();
        thread::spawn(move || {
            co.committer.name.push_str(&i.to_string());
            co.committer.name = co.to_sha1();
            let mut hash = co.to_sha1();
            let mut count: u64 = 0;
            while start.elapsed() < duration {
                co.committer.name = hash.clone();
                hash = co.to_sha1();
                count += 1;
            }
            tx.send(count).unwrap();
        });
    }
    drop(tx);
    let total: u64 = rx.iter().sum();
    let elapsed = start.elapsed().as_secs_f64();
    println!(
        "Benchmark ({} threads): {:.2}M hashes/sec  ({} hashes in {:.1}s)",
        jobs,
        total as f64 / elapsed / 1_000_000.0,
        total,
        elapsed
    );
}

/// Spawn bruteforce thread and catch the result and check it and loop back unless there are no expected result.
fn bruteforce(settings: Settings, commit_object: &CommitObject, job_count: usize) -> String {
    let mut found_hash: String = "".to_owned();
    let mut iteration_count = 0;
    let (tx, rx) = channel();
    println!();

    while found_hash.is_empty() {
        for i in 0..job_count {
            let settings: Settings = settings.clone();
            let tx = tx.clone();
            let mut co = commit_object.clone();

            thread::spawn(move || {
                co.committer.name.push_str(&(iteration_count * job_count + i).to_string());
                let mut commit_hash = co.to_sha1();

                for _ in 0..1u64 << settings.block_size {
                    co.committer.name = commit_hash.clone();
                    let pre = commit_hash.clone();
                    commit_hash = co.to_sha1();
                    if settings.patterns.iter().any(|p| commit_hash.starts_with(p.as_str())) {
                        tx.send(Some(pre)).unwrap();
                        return;
                    }
                }
                tx.send(None).unwrap();
            });
        }
        for _ in 0..job_count {
            let r = rx.recv().unwrap();
            if let Some(r) = r {
                found_hash = r;
            }
        }
        iteration_count += 1;
        println!(
            "\x1b[1A{} hashes calculated...",
            iteration_count as u128 * (1 << settings.block_size) as u128 * settings.jobs as u128
        );
    }
    found_hash
}
