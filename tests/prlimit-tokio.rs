use std::{os::unix::process::ExitStatusExt, sync::Arc};

use anyhow::Context;
use leucite::{CommandExt, MemorySize, Rules};
use tmpdir::TmpDir;
use tokio::process::Command;

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
    tokio::fs::write(
        soln,
        r#"
            #include <stdio.h>
            #include <stdlib.h>
            int main(void) {
                for (int i = 1; i < 10; ++i) {
                    char *data = malloc(1 * 1000 * 1000);
                    if (data == NULL) {
                        fprintf(stderr, "[ERR] Out of memory\n");
                        return 69;
                    }
                    data[0] = 'H';
                    data[1] = 'i';
                    data[2] = ' ';
                    data[3] = '0' + i;
                    data[4] = '\0';
                    printf("%s\n", data);
                }
                return 0;
            }
        "#,
    )
    .await?;

    Command::new("gcc")
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

    let exit = Command::new("./test")
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
