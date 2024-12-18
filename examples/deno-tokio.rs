use anyhow::Context;
use leucite::{tokio::CommandExt, Rules};
use tmpdir::TmpDir;
use tokio::process::Command;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let tempdir = TmpDir::new("leucite").await.context("creating temp dir")?;

    let rules = Rules::new()
        .add_read_only("/usr")
        .add_read_only("/etc")
        .add_read_only("/dev")
        .add_read_only("/bin")
        .add_read_write(tempdir.to_path_buf());

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
    let exit = child.wait().await?;
    println!("cmd done, exit = {:?}", exit);

    Ok(())
}
