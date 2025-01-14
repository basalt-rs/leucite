use std::{process::Command, sync::Arc};

use anyhow::Context;
use leucite::{CommandExt, MemorySize, Rules};
use tempdir::TempDir;

#[test]
fn deno_std() -> anyhow::Result<()> {
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
    soln.push("run.ts");
    std::fs::write(
        soln,
        r#"
            const file = await Deno.readTextFile('/home/user');
            console.log(file);
        "#,
    )?;

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
        .wait()?;

    assert_eq!(exit.code(), Some(1));

    Ok(())
}
