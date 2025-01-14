use std::sync::Arc;

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

    let exit = TokioCommand::new("./test")
        .current_dir(&tempdir)
        .env_clear()
        .max_memory(MemorySize::from_mb(5))
        .restrict(rules)
        .spawn()
        .context("spawning run command")?
        .wait()
        .await?;

    assert_eq!(exit.code(), Some(69)); // code is that returned by the C program

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

    let exit = StdCommand::new("./test")
        .current_dir(&tempdir)
        .env_clear()
        .max_memory(MemorySize::from_mb(5))
        .restrict(rules)
        .spawn()
        .context("spawning run command")?
        .wait()?;

    assert_eq!(exit.code(), Some(69)); // code is that returned by the C program

    Ok(())
}
