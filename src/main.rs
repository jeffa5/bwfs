use bwfs::mapfs::MapFS;

use tracing::debug;
use tracing::info;

use clap::Parser;
use fuser::MountOption;

#[derive(Debug, Parser)]
struct Args {
    /// Where to mount the secrets.
    #[clap()]
    mountpoint: String,

    #[clap(long)]
    auto_unmount: bool,
}

fn main() {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    info!(?args, "Loaded args");

    let fs = bw_init();

    // let mut fs = MapFS::new();
    // for i in 0..100 {
    //     let parent = fs.add_dir(format!("test{i:04}"));
    //     for i in 0..100 {
    //         fs.add_file(parent, format!("value{i:04}"), "password1234".to_owned());
    //     }
    // }

    info!(args.mountpoint, "Configuring mount");
    let mut mount_options = Vec::new();
    mount_options.push(MountOption::RO);
    if args.auto_unmount {
        mount_options.push(MountOption::AutoUnmount);
    }
    fuser::mount2(fs, args.mountpoint, &[MountOption::RO]).unwrap();
}

fn bw_init() -> MapFS {
    let mut cli = bwfs::client::BWCLI::new("bw".to_owned());

    let status = cli.status().unwrap();
    debug!("{:?}", status);
    if status.status != "unlocked" {
        info!("locked, unlocking");
        cli.unlock().unwrap();
    }

    info!("unlocked, listing secrets");
    let secrets = cli.list().unwrap();

    let mut fs = MapFS::new();
    for secret in secrets {
        let parent = fs.add_dir(secret.name);
        if let Some(login) = secret.login {
            if let Some(username) = login.username {
                fs.add_file(parent, "username".to_owned(), username);
            }
            if let Some(password) = login.password {
                fs.add_file(parent, "password".to_owned(), password);
            }
            if let Some(uris) = login.uris {
                for (i, uri) in uris.into_iter().enumerate() {
                    let uri_name = format!("uri{i}");
                    fs.add_file(parent, uri_name, uri.uri);
                }
            }
        }
    }
    fs
}
