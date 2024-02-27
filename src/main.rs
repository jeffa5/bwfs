use std::collections::BTreeMap;
use std::time::SystemTime;

use bwfs::mapfs::MapFS;

use serde::de::value::StringDeserializer;
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
        let parts: Vec<_> = folder.name.split('/').collect();
        let mut parent = 1;
        let mut name = folder.name.clone();
        if parts.len() > 1 {
            // has parents, ensure they exist or add them
            let keep = parts.len() - 1;
            for part in parts.iter().take(keep) {
                match fs.find(parent, (*part).to_owned()) {
                    Some(p) => parent = p,
                    None => {
                        parent = fs.add_dir(
                            parent,
                            (*part).to_owned(),
                            SystemTime::now(),
                            SystemTime::now(),
                        )
                    }
                }
            }
            name = parts[keep].to_owned();
        }
        let inode = fs.add_dir(parent, name, SystemTime::now(), SystemTime::now());
        folders_map.insert(folder.id.unwrap_or_default(), inode);
    }

    for secret in secrets {
        let folder_id = folders_map
            .get(&secret.folder_id.unwrap_or_default())
            .unwrap();
        let ctime = SystemTime::from(
            time::serde::rfc3339::deserialize(StringDeserializer::<serde::de::value::Error>::new(
                secret.creation_date,
            ))
            .unwrap(),
        );
        let mtime = SystemTime::from(
            time::serde::rfc3339::deserialize(StringDeserializer::<serde::de::value::Error>::new(
                secret.revision_date,
            ))
            .unwrap(),
        );
        let parent = fs.add_dir(*folder_id, secret.name, ctime.clone(), mtime.clone());
        if let Some(login) = secret.login {
            if let Some(username) = login.username {
                fs.add_file(
                    parent,
                    "username".to_owned(),
                    username,
                    ctime.clone(),
                    mtime.clone(),
                );
            }
            if let Some(password) = login.password {
                fs.add_file(
                    parent,
                    "password".to_owned(),
                    password,
                    ctime.clone(),
                    mtime.clone(),
                );
            }
            if let Some(uris) = login.uris {
                if !uris.is_empty() {
                    let uris_dir = fs.add_dir(parent, "uris".to_owned(), ctime, mtime);
                    for (i, uri) in uris.into_iter().enumerate() {
                        fs.add_file(
                            uris_dir,
                            format!("{:02}", i + 1),
                            uri.uri,
                            ctime.clone(),
                            mtime.clone(),
                        );
                    }
                }
            }
        }
        if let Some(notes) = secret.notes {
            fs.add_file(
                parent,
                "notes".to_owned(),
                notes,
                ctime.clone(),
                mtime.clone(),
            );
        }
        if let Some(fields) = secret.fields {
            if !fields.is_empty() {
                let fields_dir = fs.add_dir(parent, "fields".to_owned(), ctime, mtime);
                for field in fields {
                    fs.add_file(
                        fields_dir,
                        field.name,
                        field.value,
                        ctime.clone(),
                        mtime.clone(),
                    );
                }
            }
        }
        fs.add_file(
            parent,
            "id".to_owned(),
            secret.id,
            ctime.clone(),
            mtime.clone(),
        );
    }
    fs
}
