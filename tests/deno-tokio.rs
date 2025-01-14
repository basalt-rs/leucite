use std::sync::Arc;

use anyhow::Context;
use leucite::{CommandExt, MemorySize, Rules};
use tmpdir::TmpDir;
use tokio::process::Command;

#[tokio::test]
async fn deno_tokio() -> anyhow::Result<()> {
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
    soln.push("run.ts");
    tokio::fs::write(
        soln,
        r#"
            const file = await Deno.readTextFile('/home/user');
            console.log(file);
        "#,
    )
    .await?;

    let exit = Command::new("deno")
        .arg("run")
        .arg("-A")
        .arg("run.ts")
        .current_dir(&tempdir)
        .env_clear()
        .restrict(rules)
        .max_memory(MemorySize::from_gb(1)) // Deno seems to require a lot of memory
        .spawn()
        .context("spawning command")?
        .wait()
        .await?;

    assert_eq!(exit.code(), Some(1));

    Ok(())
}
