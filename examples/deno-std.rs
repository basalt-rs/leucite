use std::process::Command;

use anyhow::Context;
use leucite::{CommandExt, Rules};
use tempdir::TempDir;

fn main() -> anyhow::Result<()> {
    let tempdir = TempDir::new("leucite").context("creating temp dir")?;

    let rules = Rules::new()
        .add_read_only("/usr")
        .add_read_only("/etc")
        .add_read_only("/dev")
        .add_read_only("/bin")
        .add_read_write(tempdir.path());

    let mut soln = tempdir.path().to_path_buf();
    soln.push("run.ts");
    std::fs::write(
        soln,
        r#"
            const file = await Deno.readTextFile('/home/user');
            console.log(file);
        "#,
    )?;

    dbg!(&tempdir);
    let mut child = Command::new("deno")
        .arg("run")
        .arg("-A")
        .arg("run.ts")
        .current_dir(&tempdir)
        .env_clear()
        .spawn_restricted(rules)
        .context("spawning command")?;

    println!("running command...");
    let exit = child.wait()?;
    println!("cmd done, exit = {:?}", exit);

    Ok(())
}
