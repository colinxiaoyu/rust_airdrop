use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "airdrop")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Start,
    Status,
    Stop,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start => {
            println!("Starting airdropd...");
            // 后面这里换成 systemd / launchd
        }
        Commands::Status => {
            println!("airdropd is running");
        }
        Commands::Stop => {
            println!("Stopping airdropd...");
        }
    }
}
