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
struct Opts {
    #[clap(long, global = true, default_value = "/tmp/bwfs")]
    socket: String,

    #[clap(subcommand)]
    cmd: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Serve the filesystem.
    Serve(ServeArgs),

    Unlock,

    Lock,

    Status,

    Refresh,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Opts::parse();
    info!(?args, "Loaded args");

    match args.cmd {
        Command::Serve(serve_args) => serve(args.socket, serve_args),
        Command::Unlock => unlock(args.socket),
        Command::Lock => lock(args.socket),
        Command::Status => status(args.socket),
        Command::Refresh => refresh(args.socket),
    }
}
