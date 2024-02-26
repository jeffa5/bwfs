use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    process::{Command, Stdio},
    time::{Duration, SystemTime},
};

use clap::Parser;
use fuser::{FileAttr, FileType, Filesystem, MountOption};
use libc::ENOENT;

#[derive(Debug, Parser)]
struct Args {
    /// Where to mount the secrets.
    #[clap()]
    mountpoint: String,
}

#[derive(Debug)]
pub enum FSEntry {
    Dir(BTreeMap<String, u64>),
    File(String),
}

impl FSEntry {
    fn attrs(&self, ino: u64) -> FileAttr {
        FileAttr {
            ino,
            size: self.size(),
            blocks: 1,
            atime: SystemTime::now(),
            mtime: SystemTime::now(),
            ctime: SystemTime::now(),
            crtime: SystemTime::now(),
            kind: self.kind(),
            perm: 0o755,
            nlink: 1,
            uid: 0,
            gid: 0,
            rdev: 1,
            blksize: 1024,
            flags: 0,
        }
    }

    fn kind(&self) -> FileType {
        match self {
            FSEntry::Dir(_) => FileType::Directory,
            FSEntry::File(_) => FileType::RegularFile,
        }
    }

    fn size(&self) -> u64 {
        match self {
            FSEntry::Dir(_) => 0,
            FSEntry::File(content) => content.as_bytes().len() as u64,
        }
    }
}

fn sanitize_name(name: &str) -> String {
    pub const PROHIBITED_PATH_CHARS: &[char] =
        &['/', '\\', '?', '%', '*', ':', '|', '"', '<', '>', '.'];
    name.replace(PROHIBITED_PATH_CHARS, "")
}

#[derive(Debug)]
pub struct MapFS {
    name_map: BTreeMap<(u64, String), u64>,
    inode_map: BTreeMap<u64, FSEntry>,
    handles: BTreeMap<u64, u64>,
    generation: u64,
}

impl MapFS {
    pub fn new() -> Self {
        let mut s = Self {
            name_map: BTreeMap::new(),
            inode_map: BTreeMap::new(),
            handles: BTreeMap::new(),
            generation: 1,
        };
        s.inode_map.insert(1, FSEntry::Dir(BTreeMap::new()));
        s
    }

    fn next_id(&self) -> u64 {
        self.inode_map.keys().max().copied().unwrap_or_default() + 1
    }

    pub fn add_dir(&mut self, name: String) -> u64 {
        let name = sanitize_name(&name);
        let inode = self.next_id();
        let parent = 1;
        if let Some(FSEntry::Dir(children)) = self.inode_map.get_mut(&parent) {
            children.insert(name.clone(), inode);
        }
        self.name_map.insert((parent, name), inode);
        self.inode_map.insert(inode, FSEntry::Dir(BTreeMap::new()));
        inode
    }

    pub fn add_file(&mut self, parent: u64, name: String, value: String) -> u64 {
        let name = sanitize_name(&name);
        let inode = self.next_id();
        if let Some(FSEntry::Dir(children)) = self.inode_map.get_mut(&parent) {
            children.insert(name.clone(), inode);
        }
        self.name_map.insert((parent, name), inode);
        self.inode_map.insert(inode, FSEntry::File(value));
        inode
    }

    pub fn register_fh(&mut self, ino: u64) -> u64 {
        let new_fh = self.handles.values().max().copied().unwrap_or_default() + 1;
        *self.handles.entry(ino).or_insert(new_fh)
    }
}

impl Filesystem for MapFS {
    fn lookup(
        &mut self,
        _req: &fuser::Request<'_>,
        parent: u64,
        name: &std::ffi::OsStr,
        reply: fuser::ReplyEntry,
    ) {
        let name = name.to_str().unwrap();
        println!("lookup: {} {}", parent, name);
        if let Some(ino) = self.name_map.get(&(parent, name.to_owned())).copied() {
            let entry = self.inode_map.get(&ino).unwrap();
            println!("looked up secret {}", name);
            let attrs = entry.attrs(ino);
            dbg!(&attrs);
            reply.entry(&Duration::from_secs(60), &attrs, self.generation)
        } else {
            println!("didn't find lookup for {name}");
            reply.error(ENOENT)
        }
    }

    fn opendir(
        &mut self,
        _req: &fuser::Request<'_>,
        ino: u64,
        flags: i32,
        reply: fuser::ReplyOpen,
    ) {
        println!("opendir: {} {}", ino, flags);
        if let Some(entry) = self.inode_map.get(&ino) {
            let fh = self.register_fh(ino);
            reply.opened(fh, 0)
        } else {
            reply.error(ENOENT)
        }
    }

    fn getattr(&mut self, _req: &fuser::Request<'_>, ino: u64, reply: fuser::ReplyAttr) {
        println!("getattr: {}", ino);
        if let Some(entry) = self.inode_map.get(&ino) {
            println!("Found entry");
            let attrs = entry.attrs(ino);
            dbg!(&attrs);
            reply.attr(&Duration::from_secs(60), &attrs);
        } else {
            println!("Failed to find entry");
            reply.error(ENOENT)
        }
    }

    fn readdir(
        &mut self,
        _req: &fuser::Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        mut reply: fuser::ReplyDirectory,
    ) {
        println!("readdir: {ino} {fh} {offset}");
        if !self.handles.contains_key(&ino) {
            reply.error(ENOENT);
            return;
        }
        if let Some(FSEntry::Dir(children)) = self.inode_map.get(&ino) {
            for (i, (name, id)) in children.iter().enumerate().skip(offset as usize) {
                let child = self.inode_map.get(id).unwrap();
                println!("adding {}", name);
                let full = reply.add(*id, i as i64 + 1, child.kind(), name);
                if full {
                    println!("readdir full");
                    break;
                }
            }
            reply.ok()
        } else {
            reply.error(ENOENT)
        }
    }

    fn open(&mut self, _req: &fuser::Request<'_>, ino: u64, flags: i32, reply: fuser::ReplyOpen) {
        println!("open: {ino} {flags}");
        if let Some(FSEntry::File(content)) = self.inode_map.get(&ino) {
            let fh = self.register_fh(ino);
            reply.opened(fh, 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn read(
        &mut self,
        _req: &fuser::Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        size: u32,
        flags: i32,
        lock_owner: Option<u64>,
        reply: fuser::ReplyData,
    ) {
        println!("read: {ino} {fh} {offset} {size}");
        if let Some(FSEntry::File(content)) = self.inode_map.get(&ino) {
            reply.data(content.as_bytes());
        } else {
            reply.error(ENOENT);
        }
    }
}

fn main() {
    env_logger::init();
    let args = Args::parse();
    println!("{:?}", args);

    let fs = bw_init();

    // let mut fs = MapFS::new();
    // for i in 0..100 {
    //     let parent = fs.add_dir(format!("test{i:04}"));
    //     for i in 0..100 {
    //         fs.add_file(parent, format!("value{i:04}"), "password1234".to_owned());
    //     }
    // }

    println!("{:?}", fs);

    println!("Configuring mount");
    fuser::mount2(fs, args.mountpoint, &[MountOption::RO]).unwrap();
}

fn bw_init() -> MapFS {
    let mut cli = BWCLI {
        path: "bw".to_owned(),
        session_token: None,
    };

    let status = cli.status().unwrap();
    println!("{:?}", status);
    if status.status != "unlocked" {
        println!("locked, unlocking");
        cli.unlock().unwrap();
    }

    println!("unlocked, listing secrets");
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

pub struct BWCLI {
    path: String,
    session_token: Option<String>,
}

impl BWCLI {
    fn command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new(&self.path);
        cmd.args(args);
        println!("Executing command {:?}", cmd);
        if let Some(session_token) = &self.session_token {
            cmd.env("BW_SESSION", session_token);
        }
        cmd
    }

    pub fn status(&self) -> Result<Status, String> {
        let output = self
            .command(&["status"])
            .output()
            .map_err(|e| e.to_string())?;
        let stdout = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
        let status: Status = serde_json::from_str(&stdout).map_err(|e| e.to_string())?;
        Ok(status)
    }

    pub fn unlock(&mut self) -> Result<(), String> {
        let output = self
            .command(&["unlock", "--raw"])
            .stdin(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| e.to_string())?;
        let session_token = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
        self.session_token = Some(session_token);
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<Secret>, String> {
        let output = self
            .command(&["list", "items"])
            .output()
            .map_err(|e| e.to_string())?;
        let stdout = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
        let secrets_list: Vec<Secret> = serde_json::from_str(&stdout).map_err(|e| e.to_string())?;
        Ok(secrets_list)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    last_sync: String,
    user_email: String,
    user_id: String,
    status: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Secret {
    password_history: Option<Vec<SecretPasswordHistory>>,
    revision_date: String,
    creation_date: String,
    deleted_date: Option<String>,
    object: String,
    id: String,
    organization_id: Option<String>,
    folder_id: Option<String>,
    r#type: u32,
    reprompt: u32,
    name: String,
    notes: Option<String>,
    favorite: bool,
    login: Option<SecretLogin>,
    collection_ids: Vec<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretLogin {
    fido_2_credentials: Vec<String>,
    uris: Option<Vec<SecretLoginUri>>,
    username: Option<String>,
    password: Option<String>,
    totp: Option<String>,
    password_revision_date: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretLoginUri {
    r#match: Option<String>,
    uri: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretPasswordHistory {
    last_used_date: String,
    password: String,
}
