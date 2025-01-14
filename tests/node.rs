use std::sync::Arc;

use anyhow::Context;
use leucite::{CommandExt, MemorySize, Rules};
use std::process::Command as StdCommand;
use tempdir::TempDir;
use tmpdir::TmpDir;
use tokio::process::Command as TokioCommand;

#[tokio::test]
async fn node_tokio() -> anyhow::Result<()> {
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
    soln.push("run.js");
    tokio::fs::write(
        soln,
        r#"
            const fs = require('fs');
            const file = fs.readFileSync('/home/user', 'utf-8');
            console.log(file);
        "#,
    )
    .await?;

    let exit = TokioCommand::new("node")
        .arg("run.js")
        .current_dir(&tempdir)
        .env_clear()
        .restrict(rules)
        .max_memory(MemorySize::from_gb(1)) // Man, these javascript runtimes use a lot of memory...
        .spawn()
        .context("spawning command")?
        .wait()
        .await?;

    assert_ne!(exit.code(), Some(0));

    tempdir.close().await?;

    Ok(())
}

#[test]
fn node_std() -> anyhow::Result<()> {
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
    soln.push("run.js");
    std::fs::write(
        soln,
        r#"
            const fs = require('fs');
            const file = fs.readFileSync('/home/user', 'utf-8');
            console.log(file);
        "#,
    )?;

    let exit = StdCommand::new("node")
        .arg("run.js")
        .current_dir(&tempdir)
        .env_clear()
        .restrict(rules)
        .max_memory(MemorySize::from_gb(1)) // Man, these javascript runtimes use a lot of memory...
        .spawn()
        .context("spawning command")?
        .wait()?;

    assert_ne!(exit.code(), Some(0));

    Ok(())
}
