use crate::Cli;
use clap::CommandFactory;
use clap_complete::generate;
use std::{env, io, path::Path};

/// Generate shell completion
#[derive(Debug, clap::Parser)]
#[command(rename_all_env = "SNAKE_CASE")]
pub struct GetCompletion {
    /// The shell to generate completions for
    #[arg(value_enum)]
    shell: Shell,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
#[value(rename_all = "lowercase")]
enum Shell {
    /// Bourne Again `SHell` (bash)
    Bash,
    /// Elvish shell
    Elvish,
    /// Friendly Interactive `SHell` (fish)
    Fish,
    /// `PowerShell`
    #[value(alias = "ps")]
    #[allow(clippy::enum_variant_names)]
    PowerShell,
    /// Z `SHell` (zsh)
    Zsh,
}

impl From<Shell> for clap_complete::Shell {
    fn from(value: Shell) -> Self {
        match value {
            Shell::Bash => Self::Bash,
            Shell::Elvish => Self::Elvish,
            Shell::Fish => Self::Fish,
            Shell::PowerShell => Self::PowerShell,
            Shell::Zsh => Self::Zsh,
        }
    }
}

impl GetCompletion {
    pub async fn run(self) -> anyhow::Result<()> {
        let mut cmd = Cli::command();
        let bin_name = env::args()
            .next()
            .and_then(|path| {
                Path::new(&path)
                    .file_stem()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| env!("CARGO_BIN_NAME").to_string());

        generate(
            clap_complete::Shell::from(self.shell),
            &mut cmd,
            &bin_name,
            &mut io::stdout(),
        );

        Ok(())
    }
}
