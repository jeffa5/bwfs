use bwfs::client::lock;
use bwfs::client::refresh;
use bwfs::client::status;
use bwfs::client::unlock;
use bwfs::server::serve;
use bwfs::server::ServeArgs;
use clap::Subcommand;
use tracing::info;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "BWFS")]
#[command(version = "0.1.0")]
#[command(about = "A bitwarden FUSE filesystem")]
struct Opts {
    /// Socket to connect to the server on.
    #[clap(long, global = true, default_value = "/tmp/bwfs")]
    socket: String,

    #[clap(subcommand)]
    cmd: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Serve the filesystem.
    Serve(ServeArgs),

    /// Unlock the vault.
    Unlock {
        /// Do not refresh the filesystem contents after unlocking.
        #[clap(long)]
        no_refresh: bool,

        /// Custom password prompt script.
        ///
        /// Must output the password onto stdout, stderr will be presented to the user.
        #[clap(long)]
        password_prompt: Option<String>,
    },

    /// Lock the vault.
    Lock,

    /// Get the status of the filesystem.
    Status,

    /// Refresh the contents of the filesystem from the vault.
    Refresh,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Opts::parse();
    info!(?args, "Loaded args");

    match args.cmd {
        Command::Serve(serve_args) => serve(args.socket, serve_args),
        Command::Unlock {
            no_refresh,
            password_prompt,
        } => unlock(args.socket, no_refresh, password_prompt),
        Command::Lock => lock(args.socket),
        Command::Status => {
            let exit_code = status(args.socket)?;
            std::process::exit(exit_code)
        }
        Command::Refresh => refresh(args.socket),
    }
}
