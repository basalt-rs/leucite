use std::{process::Stdio, sync::Arc};

use anyhow::Context;
use leucite::{CommandExt, MemorySize, Rules};
use std::process::Command as StdCommand;
use tempdir::TempDir;
use tmpdir::TmpDir;
use tokio::process::Command as TokioCommand;

#[tokio::test]
async fn prlimit_tokio() -> anyhow::Result<()> {
    let tempdir = TmpDir::new("leucite").await.context("creating temp dir")?;

    let rules = Arc::new(
        Rules::new()
            .add_read_only("/usr")
            .add_read_only("/etc")
            .add_read_only("/dev")
            .add_read_only("/bin")
            .add_read_write(tempdir.to_path_buf()),
    );

    let mut soln = tempdir.to_path_buf();
    soln.push("test.c");
    tokio::fs::write(soln, include_str!("./prlimit-test.c")).await?;

    TokioCommand::new("gcc")
        .arg("-o")
        .arg("test")
        .arg("test.c")
        .arg("-save-temps")
        .current_dir(&tempdir)
        .restrict(Arc::clone(&rules))
        .spawn()
        .context("spawning compile command")?
        .wait()
        .await?;

    let out = TokioCommand::new("./test")
        .current_dir(&tempdir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear()
        .max_memory(MemorySize::from_mb(5))
        .restrict(rules)
        .spawn()
        .context("spawning run command")?
        .wait_with_output()
        .await?;

    // capture the stdout/sterr so that it is not logged when the test succeeds
    let stdout = String::from_utf8_lossy(&out.stdout);
    stdout.lines().for_each(|l| println!("[STDOUT] {}", l));

    let stderr = String::from_utf8_lossy(&out.stderr);
    stderr.lines().for_each(|l| println!("[STDERR] {}", l));

    assert_eq!(out.status.code(), Some(69)); // code is that returned by the C program

    tempdir.close().await?;

    Ok(())
}

#[test]
fn prlimit_std() -> anyhow::Result<()> {
    let tempdir = TempDir::new("leucite").context("creating temp dir")?;

    let rules = Arc::new(
        Rules::new()
            .add_read_only("/usr")
            .add_read_only("/etc")
            .add_read_only("/dev")
            .add_read_only("/bin")
            .add_read_write(tempdir.path()),
    );

    let mut soln = tempdir.path().to_path_buf();
    soln.push("test.c");
    std::fs::write(soln, include_str!("./prlimit-test.c"))?;

    StdCommand::new("gcc")
        .arg("-o")
        .arg("test")
        .arg("test.c")
        .arg("-save-temps")
        .current_dir(&tempdir)
        .restrict(Arc::clone(&rules))
        .spawn()
        .context("spawning compile command")?
        .wait()?;

    let out = StdCommand::new("./test")
        .current_dir(&tempdir)
        .env_clear()
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .max_memory(MemorySize::from_mb(5))
        .restrict(rules)
        .spawn()
        .context("spawning run command")?
        .wait_with_output()?;

    // capture the stdout/sterr so that it is not logged when the test succeeds
    let stdout = String::from_utf8_lossy(&out.stdout);
    stdout.lines().for_each(|l| println!("[STDOUT] {}", l));

    let stderr = String::from_utf8_lossy(&out.stderr);
    stderr.lines().for_each(|l| println!("[STDERR] {}", l));

    assert_eq!(out.status.code(), Some(69)); // code is that returned by the C program

    Ok(())
}
