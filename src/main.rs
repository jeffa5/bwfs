use std::collections::BTreeMap;

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

    /// Prevent auto unmounting to avoid errors from not being able to set `allow_other`.
    #[clap(long)]
    no_auto_unmount: bool,

    /// Path to the bw binary.
    #[clap(long, default_value = "bw")]
    bw_bin: String,
}

fn main() {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    info!(?args, "Loaded args");

    let fs = bw_init(args.bw_bin);

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
    if !args.no_auto_unmount {
        mount_options.push(MountOption::AutoUnmount);
    }
    println!("Mount configured");
    fuser::mount2(fs, args.mountpoint, &mount_options).unwrap();
}

fn bw_init(bw_bin: String) -> MapFS {
    let mut cli = bwfs::client::BWCLI::new(bw_bin);

    println!("Checking vault status");
    let status = cli.status().unwrap();
    debug!("{:?}", status);
    if status.status != "unlocked" {
        println!("Vault is locked, unlocking");
        cli.unlock().unwrap();
    }

    println!("Vault is unlocked, listing folders");
    let folders = cli.list_folders().unwrap();
    println!("Vault is unlocked, listing secrets");
    let secrets = cli.list_secrets().unwrap();

    println!("Converting secrets to filesystem");
    let mut fs = MapFS::new();

    let mut folders_map = BTreeMap::new();
    for folder in folders {
        let inode = fs.add_dir(1, folder.name);
        folders_map.insert(folder.id.unwrap_or_default(), inode);
    }

    for secret in secrets {
        let folder_id = folders_map.get(&secret.folder_id.unwrap_or_default()).unwrap();
        let parent = fs.add_dir(*folder_id, secret.name);
        if let Some(login) = secret.login {
            if let Some(username) = login.username {
                fs.add_file(parent, "username".to_owned(), username);
            }
            if let Some(password) = login.password {
                fs.add_file(parent, "password".to_owned(), password);
            }
            if let Some(uris) = login.uris {
                for (i, uri) in uris.into_iter().enumerate() {
                    let uri_name = format!("uri_{i}");
                    fs.add_file(parent, uri_name, uri.uri);
                }
            }
        }
        if let Some(notes) = secret.notes {
            fs.add_file(parent, "notes".to_owned(), notes);
        }
        if let Some(fields) = secret.fields {
            for field in fields {
                fs.add_file(parent, format!("field_{}", field.name), field.value);
            }
        }
    }
    fs
}
