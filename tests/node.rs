use std::{process::Stdio, sync::Arc};

use leucite::{CommandExt, MemorySize, Rules};
use std::process::Command as StdCommand;
use tempdir::TempDir;
use tmpdir::TmpDir;
use tokio::process::Command as TokioCommand;

#[tokio::test]
async fn node_tokio() -> Result<(), Box<dyn std::error::Error>> {
    let tempdir = TmpDir::new("leucite").await?;

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

    let out = TokioCommand::new("node")
        .arg("run.js")
        .current_dir(&tempdir)
        .env_clear()
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .restrict(
            Rules::new()
                .add_read_only("/usr")
                .add_read_only("/etc")
                .add_read_only("/dev")
                .add_read_only("/bin")
                .add_read_write(tempdir.to_path_buf())
                .into(),
        )
        .max_memory(MemorySize::from_gb(1)) // Man, these javascript runtimes use a lot of memory...
        .spawn()?
        .wait_with_output()
        .await?;

    // capture the stdout/sterr so that it is not logged when the test succeeds
    let stdout = String::from_utf8_lossy(&out.stdout);
    stdout.lines().for_each(|l| println!("[STDOUT] {}", l));

    let stderr = String::from_utf8_lossy(&out.stderr);
    stderr.lines().for_each(|l| println!("[STDERR] {}", l));

    assert_ne!(out.status.code(), Some(0));

    tempdir.close().await?;

    Ok(())
}

#[test]
fn node_std() -> Result<(), Box<dyn std::error::Error>> {
    let tempdir = TempDir::new("leucite")?;

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

    let out = StdCommand::new("node")
        .arg("run.js")
        .current_dir(&tempdir)
        .env_clear()
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .restrict(rules)
        .max_memory(MemorySize::from_gb(1)) // Man, these javascript runtimes use a lot of memory...
        .spawn()?
        .wait_with_output()?;

    // capture the stdout/sterr so that it is not logged when the test succeeds
    let stdout = String::from_utf8_lossy(&out.stdout);
    stdout.lines().for_each(|l| println!("[STDOUT] {}", l));

    let stderr = String::from_utf8_lossy(&out.stderr);
    stderr.lines().for_each(|l| println!("[STDERR] {}", l));

    assert_ne!(out.status.code(), Some(0));

    Ok(())
}
