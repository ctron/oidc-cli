use crate::Cli;
use clap::CommandFactory;
use clap_complete::generate;
use clap_complete::Shell::{Bash, Fish, Zsh};
use std::path::Path;
use std::{env, io};

/// Generate shell completion
#[derive(Debug, clap::Parser)]
#[command(rename_all_env = "SNAKE_CASE")]
pub struct GetCompletion {
    /// The shell to generate completions for. Supported values are bash, zsh or fish
    pub shell: String,
}

impl GetCompletion {
    pub async fn run(self) -> anyhow::Result<()> {
        let shell = self.shell;
        let mut cmd = Cli::command();
        let bin_name = env::args()
            .next()
            .and_then(|path| {
                Path::new(&path)
                    .file_stem()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .unwrap();

        match shell.as_str() {
            "bash" => generate(Bash, &mut cmd, &bin_name, &mut io::stdout()),
            "zsh" => generate(Zsh, &mut cmd, &bin_name, &mut io::stdout()),
            "fish" => generate(Fish, &mut cmd, &bin_name, &mut io::stdout()),
            _ => eprintln!("Unsupported shell: {}", shell),
        }
        Ok(())
    }
}
