mod chat;
mod llm;

use clap::{Parser, Subcommand};
use color_eyre::{Result, Section};

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Send a message to the AI model, or open an interactive session if no message is supplied.
    Chat {
        /// The message to send to the AI to receive a single answer for
        #[arg(short, long)]
        message: Option<String>,
        /// Simply print the answer from the AI with no fancy stuff
        #[arg(short, long, default_value_t = false)]
        plain_output: bool,
    },
}

fn chat(message: Option<String>, plain_output: bool) -> Result<()> {
    match message {
        Some(msg) => chat::single_prompt(&msg, plain_output),
        None => chat::session(),
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;

    dotenv::dotenv()
        .section("Occured when intitializing dotenv")
        .note("maybe you need to create a .env file")?;

    let cli = Cli::parse();

    match cli.command {
        Commands::Chat {
            message,
            plain_output,
        } => chat(message, plain_output),
    }
}
