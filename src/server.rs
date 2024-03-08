use std::{collections::BTreeMap, time::SystemTime};

use clap::Args;
use fuser::MountOption;
use sysinfo::{Groups, Pid, Users};
use tracing::{debug, info};
use uuid::Uuid;

use crate::{client::Secret, mapfs::MapFS};

pub mod bwclient;

#[derive(Debug, Args)]
pub struct ServeArgs {
    /// Where to mount the secrets.
    #[clap()]
    mountpoint: String,

    /// Prevent auto unmounting to avoid errors from not being able to set `allow_other`.
    #[clap(long)]
    no_auto_unmount: bool,

    /// Path to the bw binary.
    #[clap(long, default_value = "bw")]
    bw_bin: String,

    /// Filter results to those in the folders listed.
    #[clap(long, value_delimiter = ',')]
    folders: Vec<String>,

    /// User to own the filesystem entries.
    #[clap(short, long)]
    user: Option<String>,

    /// Group to own the filesystem entries.
    #[clap(short, long)]
    group: Option<String>,

    /// File access controls, in octal form.
    #[clap(short, long, default_value = "440")]
    mode: String,
}

pub fn serve(args: ServeArgs) {
    let fs = bw_init(&args);
    info!(args.mountpoint, "Configuring mount");
    let mut mount_options = Vec::new();
    mount_options.push(MountOption::RO);
    if !args.no_auto_unmount {
        mount_options.push(MountOption::AutoUnmount);
        mount_options.push(MountOption::AllowOther);
    }
    println!("Mount configured");
    fuser::mount2(fs, args.mountpoint, &mount_options).unwrap();
}

fn bw_init(args: &ServeArgs) -> MapFS {
    let mut cli = crate::client::BWCLI::new(args.bw_bin.clone());

    println!("Checking vault status");
    let status = cli.status().unwrap();
    debug!("{:?}", status);
    if status.status != "unlocked" {
        println!("Vault is locked, unlocking");
        cli.unlock().unwrap();
    }

    println!("Vault is unlocked, listing folders");
    let folders = cli.list_folders().unwrap();
    let folders = folders
        .into_iter()
        .filter(|f| args.folders.iter().any(|af| f.name.starts_with(af)))
        .collect::<Vec<_>>();
    println!("Vault is unlocked, listing secrets");
    let mut secrets = cli.list_secrets().unwrap();

    println!("Filtering secrets");
    let original_len = secrets.len();
    let folder_ids = folders.iter().map(|f| f.id.unwrap_or_default()).collect();
    filter_folders(folder_ids, &mut secrets);
    let new_len = secrets.len();
    info!(original_len, new_len, "Filtered secrets");

    let uid = if let Some(user) = &args.user {
        let users = Users::new_with_refreshed_list();
        if let Some(user) = users.iter().find(|u| u.name() == user).map(|u| u.id()) {
            user.clone()
        } else {
            panic!("Couldn't find user {user}");
        }
    } else {
        let s = sysinfo::System::new_all();
        let self_pid = std::process::id();
        s.process(Pid::from_u32(self_pid))
            .unwrap()
            .user_id()
            .unwrap()
            .clone()
    };
    let gid = if let Some(group) = &args.group {
        let groups = Groups::new_with_refreshed_list();
        if let Some(group) = groups.iter().find(|g| g.name() == group).map(|g| g.id()) {
            *group
        } else {
            panic!("Couldn't find group {group}");
        }
    } else {
        let s = sysinfo::System::new_all();
        let self_pid = std::process::id();
        s.process(Pid::from_u32(self_pid))
            .unwrap()
            .group_id()
            .unwrap()
    };
    let mode = u16::from_str_radix(&args.mode, 8).unwrap();

    println!("Converting secrets to filesystem");
    let mut fs = MapFS::new(*uid, *gid, mode);

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
        let ctime = SystemTime::from(secret.creation_date);
        let mtime = SystemTime::from(secret.revision_date);
        let parent = fs.add_dir(*folder_id, secret.name, ctime, mtime);
        fs.add_file(
            parent,
            "type".to_owned(),
            secret.r#type.to_string(),
            ctime,
            mtime,
        );
        if let Some(login) = secret.login {
            if let Some(username) = login.username {
                fs.add_file(parent, "username".to_owned(), username, ctime, mtime);
            }
            if let Some(password) = login.password {
                fs.add_file(parent, "password".to_owned(), password, ctime, mtime);
            }
            if let Some(uris) = login.uris {
                if !uris.is_empty() {
                    let uris_dir = fs.add_dir(parent, "uris".to_owned(), ctime, mtime);
                    for (i, uri) in uris.into_iter().enumerate() {
                        fs.add_file(uris_dir, format!("{:02}", i + 1), uri.uri, ctime, mtime);
                    }
                }
            }
        }
        if let Some(notes) = secret.notes {
            fs.add_file(parent, "notes".to_owned(), notes, ctime, mtime);
        }
        if let Some(fields) = secret.fields {
            if !fields.is_empty() {
                let fields_dir = fs.add_dir(parent, "fields".to_owned(), ctime, mtime);
                for field in fields {
                    fs.add_file(fields_dir, field.name, field.value, ctime, mtime);
                }
            }
        }
        fs.add_file(parent, "id".to_owned(), secret.id.to_string(), ctime, mtime);
    }
    fs
}

fn filter_folders(folder_ids: Vec<Uuid>, secrets: &mut Vec<Secret>) {
    secrets.retain(|s| folder_ids.contains(&s.folder_id.unwrap_or_default()))
}
