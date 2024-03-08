use bwfs::client::refresh;
use bwfs::client::status;
use bwfs::client::unlock;
use bwfs::client::lock;
use bwfs::server::serve;
use bwfs::server::ServeArgs;
use clap::Subcommand;
use tracing::info;

use clap::Parser;

#[derive(Debug, Parser)]
struct Opts {
    #[clap(subcommand)]
    cmd: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Serve the filesystem.
    Serve(ServeArgs),

    Unlock {
        #[clap(long, default_value = "/tmp/bwfs")]
        socket: String,
    },

    Lock {
        #[clap(long, default_value = "/tmp/bwfs")]
        socket: String,
    },


    Status {
        #[clap(long, default_value = "/tmp/bwfs")]
        socket: String,
    },

    Refresh {
        #[clap(long, default_value = "/tmp/bwfs")]
        socket: String,
    },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Opts::parse();
    info!(?args, "Loaded args");

    match args.cmd {
        Command::Serve(serve_args) => serve(serve_args),
        Command::Unlock { socket } => unlock(socket),
        Command::Lock { socket } => lock(socket),
        Command::Status { socket } => status(socket),
        Command::Refresh { socket } => refresh(socket),
    }
}
