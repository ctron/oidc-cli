mod create;
mod delete;
mod inspect;
mod list;
mod token;

use crate::Cli;
use clap::CommandFactory;
use clap_complete::{
    generate,
    shells::{Bash, Fish, Zsh},
};
use std::path::Path;
use std::process::ExitCode;
use std::{env, io};

#[derive(Debug, clap::Subcommand)]
pub enum Command {
    Create(create::Create),
    Delete(delete::Delete),
    Token(token::GetToken),
    List(list::List),
    Inspect(inspect::Inspect),
    Completion { shell: String },
}

impl Command {
    pub async fn run(self) -> anyhow::Result<ExitCode> {
        match self {
            Self::Create(cmd) => cmd.run().await,
            Self::Delete(cmd) => cmd.run().await,
            Self::Token(cmd) => cmd.run().await,
            Self::List(cmd) => cmd.run().await,
            Self::Inspect(cmd) => cmd.run().await,
            Self::Completion { shell } => {
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
        .map(|()| ExitCode::SUCCESS)
    }
}
