use fuser::FileAttr;
use fuser::FileType;
use fuser::Filesystem;
use libc::ENOENT;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use tracing::debug;
use tracing::info;
use uuid::Uuid;

use super::bwclient::Secret;
use super::bwclient::BWCLI;

#[derive(Clone, Debug)]
pub struct MapFSRef(pub Arc<Mutex<MapFS>>);

impl MapFSRef {
    pub fn refresh(&self, cli: &BWCLI) -> anyhow::Result<()> {
        self.0.lock().unwrap().refresh(cli)
    }
}

impl Filesystem for MapFSRef {
    fn init(
        &mut self,
        req: &fuser::Request<'_>,
        config: &mut fuser::KernelConfig,
    ) -> Result<(), libc::c_int> {
        self.0.lock().unwrap().init(req, config)
    }

    fn destroy(&mut self) {
        self.0.lock().unwrap().destroy()
    }

    fn lookup(
        &mut self,
        req: &fuser::Request<'_>,
        parent: u64,
        name: &std::ffi::OsStr,
        reply: fuser::ReplyEntry,
    ) {
        self.0.lock().unwrap().lookup(req, parent, name, reply)
    }

    fn forget(&mut self, req: &fuser::Request<'_>, ino: u64, nlookup: u64) {
        self.0.lock().unwrap().forget(req, ino, nlookup)
    }

    fn getattr(&mut self, req: &fuser::Request<'_>, ino: u64, reply: fuser::ReplyAttr) {
        self.0.lock().unwrap().getattr(req, ino, reply)
    }

    fn setattr(
        &mut self,
        req: &fuser::Request<'_>,
        ino: u64,
        mode: Option<u32>,
        uid: Option<u32>,
        gid: Option<u32>,
        size: Option<u64>,
        atime: Option<fuser::TimeOrNow>,
        mtime: Option<fuser::TimeOrNow>,
        ctime: Option<SystemTime>,
        fh: Option<u64>,
        crtime: Option<SystemTime>,
        chgtime: Option<SystemTime>,
        bkuptime: Option<SystemTime>,
        flags: Option<u32>,
        reply: fuser::ReplyAttr,
    ) {
        self.0.lock().unwrap().setattr(
            req, ino, mode, uid, gid, size, atime, mtime, ctime, fh, crtime, chgtime, bkuptime,
            flags, reply,
        )
    }

    fn readlink(&mut self, req: &fuser::Request<'_>, ino: u64, reply: fuser::ReplyData) {
        self.0.lock().unwrap().readlink(req, ino, reply)
    }

    fn mknod(
        &mut self,
        req: &fuser::Request<'_>,
        parent: u64,
        name: &std::ffi::OsStr,
        mode: u32,
        umask: u32,
        rdev: u32,
        reply: fuser::ReplyEntry,
    ) {
        self.0
            .lock()
            .unwrap()
            .mknod(req, parent, name, mode, umask, rdev, reply)
    }

    fn mkdir(
        &mut self,
        req: &fuser::Request<'_>,
        parent: u64,
        name: &std::ffi::OsStr,
        mode: u32,
        umask: u32,
        reply: fuser::ReplyEntry,
    ) {
        self.0
            .lock()
            .unwrap()
            .mkdir(req, parent, name, mode, umask, reply)
    }

    fn unlink(
        &mut self,
        req: &fuser::Request<'_>,
        parent: u64,
        name: &std::ffi::OsStr,
        reply: fuser::ReplyEmpty,
    ) {
        self.0.lock().unwrap().unlink(req, parent, name, reply)
    }

    fn rmdir(
        &mut self,
        req: &fuser::Request<'_>,
        parent: u64,
        name: &std::ffi::OsStr,
        reply: fuser::ReplyEmpty,
    ) {
        self.0.lock().unwrap().rmdir(req, parent, name, reply)
    }

    fn symlink(
        &mut self,
        req: &fuser::Request<'_>,
        parent: u64,
        link_name: &std::ffi::OsStr,
        target: &std::path::Path,
        reply: fuser::ReplyEntry,
    ) {
        self.0
            .lock()
            .unwrap()
            .symlink(req, parent, link_name, target, reply)
    }

    fn rename(
        &mut self,
        req: &fuser::Request<'_>,
        parent: u64,
        name: &std::ffi::OsStr,
        newparent: u64,
        newname: &std::ffi::OsStr,
        flags: u32,
        reply: fuser::ReplyEmpty,
    ) {
        self.0
            .lock()
            .unwrap()
            .rename(req, parent, name, newparent, newname, flags, reply)
    }

    fn link(
        &mut self,
        req: &fuser::Request<'_>,
        ino: u64,
        newparent: u64,
        newname: &std::ffi::OsStr,
        reply: fuser::ReplyEntry,
    ) {
        self.0
            .lock()
            .unwrap()
            .link(req, ino, newparent, newname, reply)
    }

    fn open(&mut self, req: &fuser::Request<'_>, ino: u64, flags: i32, reply: fuser::ReplyOpen) {
        self.0.lock().unwrap().open(req, ino, flags, reply)
    }

    fn read(
        &mut self,
        req: &fuser::Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        size: u32,
        flags: i32,
        lock_owner: Option<u64>,
        reply: fuser::ReplyData,
    ) {
        self.0
            .lock()
            .unwrap()
            .read(req, ino, fh, offset, size, flags, lock_owner, reply)
    }

    fn write(
        &mut self,
        req: &fuser::Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        data: &[u8],
        write_flags: u32,
        flags: i32,
        lock_owner: Option<u64>,
        reply: fuser::ReplyWrite,
    ) {
        self.0.lock().unwrap().write(
            req,
            ino,
            fh,
            offset,
            data,
            write_flags,
            flags,
            lock_owner,
            reply,
        )
    }

    fn flush(
        &mut self,
        req: &fuser::Request<'_>,
        ino: u64,
        fh: u64,
        lock_owner: u64,
        reply: fuser::ReplyEmpty,
    ) {
        self.0
            .lock()
            .unwrap()
            .flush(req, ino, fh, lock_owner, reply)
    }

    fn release(
        &mut self,
        req: &fuser::Request<'_>,
        ino: u64,
        fh: u64,
        flags: i32,
        lock_owner: Option<u64>,
        flush: bool,
        reply: fuser::ReplyEmpty,
    ) {
        self.0
            .lock()
            .unwrap()
            .release(req, ino, fh, flags, lock_owner, flush, reply)
    }

    fn fsync(
        &mut self,
        req: &fuser::Request<'_>,
        ino: u64,
        fh: u64,
        datasync: bool,
        reply: fuser::ReplyEmpty,
    ) {
        self.0.lock().unwrap().fsync(req, ino, fh, datasync, reply)
    }

    fn opendir(&mut self, req: &fuser::Request<'_>, ino: u64, flags: i32, reply: fuser::ReplyOpen) {
        self.0.lock().unwrap().opendir(req, ino, flags, reply)
    }

    fn readdir(
        &mut self,
        req: &fuser::Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        reply: fuser::ReplyDirectory,
    ) {
        self.0.lock().unwrap().readdir(req, ino, fh, offset, reply)
    }

    fn readdirplus(
        &mut self,
        req: &fuser::Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        reply: fuser::ReplyDirectoryPlus,
    ) {
        self.0
            .lock()
            .unwrap()
            .readdirplus(req, ino, fh, offset, reply)
    }

    fn releasedir(
        &mut self,
        req: &fuser::Request<'_>,
        ino: u64,
        fh: u64,
        flags: i32,
        reply: fuser::ReplyEmpty,
    ) {
        self.0
            .lock()
            .unwrap()
            .releasedir(req, ino, fh, flags, reply)
    }

    fn fsyncdir(
        &mut self,
        req: &fuser::Request<'_>,
        ino: u64,
        fh: u64,
        datasync: bool,
        reply: fuser::ReplyEmpty,
    ) {
        self.0
            .lock()
            .unwrap()
            .fsyncdir(req, ino, fh, datasync, reply)
    }

    fn statfs(&mut self, req: &fuser::Request<'_>, ino: u64, reply: fuser::ReplyStatfs) {
        self.0.lock().unwrap().statfs(req, ino, reply)
    }

    fn setxattr(
        &mut self,
        req: &fuser::Request<'_>,
        ino: u64,
        name: &std::ffi::OsStr,
        value: &[u8],
        flags: i32,
        position: u32,
        reply: fuser::ReplyEmpty,
    ) {
        self.0
            .lock()
            .unwrap()
            .setxattr(req, ino, name, value, flags, position, reply)
    }

    fn getxattr(
        &mut self,
        req: &fuser::Request<'_>,
        ino: u64,
        name: &std::ffi::OsStr,
        size: u32,
        reply: fuser::ReplyXattr,
    ) {
        self.0.lock().unwrap().getxattr(req, ino, name, size, reply)
    }

    fn listxattr(
        &mut self,
        req: &fuser::Request<'_>,
        ino: u64,
        size: u32,
        reply: fuser::ReplyXattr,
    ) {
        self.0.lock().unwrap().listxattr(req, ino, size, reply)
    }

    fn removexattr(
        &mut self,
        req: &fuser::Request<'_>,
        ino: u64,
        name: &std::ffi::OsStr,
        reply: fuser::ReplyEmpty,
    ) {
        self.0.lock().unwrap().removexattr(req, ino, name, reply)
    }

    fn access(&mut self, req: &fuser::Request<'_>, ino: u64, mask: i32, reply: fuser::ReplyEmpty) {
        self.0.lock().unwrap().access(req, ino, mask, reply)
    }

    fn create(
        &mut self,
        req: &fuser::Request<'_>,
        parent: u64,
        name: &std::ffi::OsStr,
        mode: u32,
        umask: u32,
        flags: i32,
        reply: fuser::ReplyCreate,
    ) {
        self.0
            .lock()
            .unwrap()
            .create(req, parent, name, mode, umask, flags, reply)
    }

    fn getlk(
        &mut self,
        req: &fuser::Request<'_>,
        ino: u64,
        fh: u64,
        lock_owner: u64,
        start: u64,
        end: u64,
        typ: i32,
        pid: u32,
        reply: fuser::ReplyLock,
    ) {
        self.0
            .lock()
            .unwrap()
            .getlk(req, ino, fh, lock_owner, start, end, typ, pid, reply)
    }

    fn setlk(
        &mut self,
        req: &fuser::Request<'_>,
        ino: u64,
        fh: u64,
        lock_owner: u64,
        start: u64,
        end: u64,
        typ: i32,
        pid: u32,
        sleep: bool,
        reply: fuser::ReplyEmpty,
    ) {
        self.0
            .lock()
            .unwrap()
            .setlk(req, ino, fh, lock_owner, start, end, typ, pid, sleep, reply)
    }

    fn bmap(
        &mut self,
        req: &fuser::Request<'_>,
        ino: u64,
        blocksize: u32,
        idx: u64,
        reply: fuser::ReplyBmap,
    ) {
        self.0.lock().unwrap().bmap(req, ino, blocksize, idx, reply)
    }

    fn ioctl(
        &mut self,
        req: &fuser::Request<'_>,
        ino: u64,
        fh: u64,
        flags: u32,
        cmd: u32,
        in_data: &[u8],
        out_size: u32,
        reply: fuser::ReplyIoctl,
    ) {
        self.0
            .lock()
            .unwrap()
            .ioctl(req, ino, fh, flags, cmd, in_data, out_size, reply)
    }

    fn fallocate(
        &mut self,
        req: &fuser::Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        length: i64,
        mode: i32,
        reply: fuser::ReplyEmpty,
    ) {
        self.0
            .lock()
            .unwrap()
            .fallocate(req, ino, fh, offset, length, mode, reply)
    }

    fn lseek(
        &mut self,
        req: &fuser::Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        whence: i32,
        reply: fuser::ReplyLseek,
    ) {
        self.0
            .lock()
            .unwrap()
            .lseek(req, ino, fh, offset, whence, reply)
    }

    fn copy_file_range(
        &mut self,
        req: &fuser::Request<'_>,
        ino_in: u64,
        fh_in: u64,
        offset_in: i64,
        ino_out: u64,
        fh_out: u64,
        offset_out: i64,
        len: u64,
        flags: u32,
        reply: fuser::ReplyWrite,
    ) {
        self.0.lock().unwrap().copy_file_range(
            req, ino_in, fh_in, offset_in, ino_out, fh_out, offset_out, len, flags, reply,
        )
    }
}

#[derive(Debug)]
pub enum FSEntry {
    Dir {
        children: BTreeMap<String, u64>,
        ctime: SystemTime,
        mtime: SystemTime,
    },
    File {
        content: String,
        ctime: SystemTime,
        mtime: SystemTime,
    },
}

impl FSEntry {
    fn attrs(&self, ino: u64, perm: u16, uid: u32, gid: u32) -> FileAttr {
        FileAttr {
            ino,
            size: self.size(),
            blocks: 1,
            atime: SystemTime::now(),
            mtime: self.mtime(),
            ctime: self.ctime(),
            crtime: SystemTime::now(),
            kind: self.kind(),
            perm,
            nlink: 1,
            uid,
            gid,
            rdev: 1,
            blksize: 1024,
            flags: 0,
        }
    }

    fn kind(&self) -> FileType {
        match self {
            FSEntry::Dir { .. } => FileType::Directory,
            FSEntry::File { .. } => FileType::RegularFile,
        }
    }

    fn size(&self) -> u64 {
        match self {
            FSEntry::Dir { .. } => 0,
            FSEntry::File { content, .. } => content.as_bytes().len() as u64,
        }
    }

    fn ctime(&self) -> SystemTime {
        match self {
            FSEntry::Dir { ctime, .. } => *ctime,
            FSEntry::File { ctime, .. } => *ctime,
        }
    }

    fn mtime(&self) -> SystemTime {
        match self {
            FSEntry::Dir { mtime, .. } => *mtime,
            FSEntry::File { mtime, .. } => *mtime,
        }
    }
}

#[derive(Debug)]
pub struct MapFS {
    name_map: BTreeMap<(u64, String), u64>,
    inode_map: BTreeMap<u64, FSEntry>,
    handles: BTreeMap<u64, u64>,
    generation: u64,
    permissions: u16,
    uid: u32,
    gid: u32,
    folders: Vec<String>,
}

impl MapFS {
    pub fn new(uid: u32, gid: u32, permissions: u16, folders: Vec<String>) -> Self {
        let mut s = Self {
            name_map: BTreeMap::new(),
            inode_map: BTreeMap::new(),
            handles: BTreeMap::new(),
            generation: 1,
            permissions,
            uid,
            gid,
            folders,
        };
        s.inode_map.insert(
            1,
            FSEntry::Dir {
                children: BTreeMap::new(),
                ctime: SystemTime::now(),
                mtime: SystemTime::now(),
            },
        );
        s
    }

    fn next_id(&self) -> u64 {
        self.inode_map.keys().max().copied().unwrap_or_default() + 1
    }

    pub fn add_dir(
        &mut self,
        parent: u64,
        name: String,
        ctime: SystemTime,
        mtime: SystemTime,
    ) -> u64 {
        let name = sanitize_name(&name);
        let inode = self.next_id();
        if let Some(FSEntry::Dir { children, .. }) = self.inode_map.get_mut(&parent) {
            children.insert(name.clone(), inode);
        }
        self.name_map.insert((parent, name), inode);
        self.inode_map.insert(
            inode,
            FSEntry::Dir {
                children: BTreeMap::new(),
                ctime,
                mtime,
            },
        );
        inode
    }

    pub fn add_file(
        &mut self,
        parent: u64,
        name: String,
        value: String,
        ctime: SystemTime,
        mtime: SystemTime,
    ) -> u64 {
        let name = sanitize_name(&name);
        let inode = self.next_id();
        if let Some(FSEntry::Dir { children, .. }) = self.inode_map.get_mut(&parent) {
            children.insert(name.clone(), inode);
        }
        self.name_map.insert((parent, name), inode);
        self.inode_map.insert(
            inode,
            FSEntry::File {
                content: value,
                ctime,
                mtime,
            },
        );
        inode
    }

    pub fn register_fh(&mut self, ino: u64) -> u64 {
        let new_fh = self.handles.values().max().copied().unwrap_or_default() + 1;
        *self.handles.entry(ino).or_insert(new_fh)
    }

    pub fn find(&self, parent: u64, name: String) -> Option<u64> {
        self.name_map.get(&(parent, name)).copied()
    }

    pub fn clear(&mut self) {
        let root_inode = self.inode_map.remove(&1).unwrap();
        *self = Self {
            name_map: Default::default(),
            inode_map: Default::default(),
            handles: Default::default(),
            generation: self.generation + 1,
            permissions: self.permissions,
            uid: self.uid,
            gid: self.gid,
            folders: std::mem::take(&mut self.folders),
        };
        self.inode_map.insert(1, root_inode);
    }

    pub fn refresh(&mut self, cli: &BWCLI) -> anyhow::Result<()> {
        if cli.status().ok().map_or(true, |s| s.status != "unlocked") {
            anyhow::bail!("BWCLI is locked");
        }

        self.clear();
        println!("Listing folders");
        let folders = cli.list_folders().unwrap();
        let folders = folders
            .into_iter()
            .filter(|f| self.folders.iter().any(|af| f.name.starts_with(af)))
            .collect::<Vec<_>>();
        println!("Vault is unlocked, listing secrets");
        let mut secrets = cli.list_secrets().unwrap();

        println!("Filtering secrets");
        let original_len = secrets.len();
        let folder_ids = folders.iter().map(|f| f.id.unwrap_or_default()).collect();
        filter_folders(folder_ids, &mut secrets);
        let new_len = secrets.len();
        info!(original_len, new_len, "Filtered secrets");

        let mut folders_map = BTreeMap::new();
        for folder in folders {
            let parts: Vec<_> = folder.name.split('/').collect();
            let mut parent = 1;
            let mut name = folder.name.clone();
            if parts.len() > 1 {
                // has parents, ensure they exist or add them
                let keep = parts.len() - 1;
                for part in parts.iter().take(keep) {
                    match self.find(parent, (*part).to_owned()) {
                        Some(p) => parent = p,
                        None => {
                            parent = self.add_dir(
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
            let inode = self.add_dir(parent, name, SystemTime::now(), SystemTime::now());
            folders_map.insert(folder.id.unwrap_or_default(), inode);
        }

        for secret in secrets {
            let folder_id = folders_map
                .get(&secret.folder_id.unwrap_or_default())
                .unwrap();
            let ctime = SystemTime::from(secret.creation_date);
            let mtime = SystemTime::from(secret.revision_date);
            let parent = self.add_dir(*folder_id, secret.name, ctime, mtime);
            self.add_file(
                parent,
                "type".to_owned(),
                secret.r#type.to_string(),
                ctime,
                mtime,
            );
            if let Some(login) = secret.login {
                if let Some(username) = login.username {
                    self.add_file(parent, "username".to_owned(), username, ctime, mtime);
                }
                if let Some(password) = login.password {
                    self.add_file(parent, "password".to_owned(), password, ctime, mtime);
                }
                if let Some(uris) = login.uris {
                    if !uris.is_empty() {
                        let uris_dir = self.add_dir(parent, "uris".to_owned(), ctime, mtime);
                        for (i, uri) in uris.into_iter().enumerate() {
                            self.add_file(uris_dir, format!("{:02}", i + 1), uri.uri, ctime, mtime);
                        }
                    }
                }
            }
            if let Some(notes) = secret.notes {
                self.add_file(parent, "notes".to_owned(), notes, ctime, mtime);
            }
            if let Some(fields) = secret.fields {
                if !fields.is_empty() {
                    let fields_dir = self.add_dir(parent, "fields".to_owned(), ctime, mtime);
                    for field in fields {
                        self.add_file(fields_dir, field.name, field.value, ctime, mtime);
                    }
                }
            }
            self.add_file(parent, "id".to_owned(), secret.id.to_string(), ctime, mtime);
        }
        Ok(())
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
        info!("lookup: {} {}", parent, name);
        if let Some(ino) = self.find(parent, name.to_owned()) {
            let entry = self.inode_map.get(&ino).unwrap();
            debug!("looked up secret {}", name);
            let attrs = entry.attrs(ino, self.permissions, self.uid, self.gid);
            reply.entry(&Duration::ZERO, &attrs, self.generation)
        } else {
            debug!("didn't find lookup for {name}");
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
        info!("opendir: {} {}", ino, flags);
        if self.inode_map.contains_key(&ino) {
            let fh = self.register_fh(ino);
            reply.opened(fh, 0)
        } else {
            reply.error(ENOENT)
        }
    }

    fn getattr(&mut self, _req: &fuser::Request<'_>, ino: u64, reply: fuser::ReplyAttr) {
        info!("getattr: {}", ino);
        if let Some(entry) = self.inode_map.get(&ino) {
            debug!("Found entry");
            let attrs = entry.attrs(ino, self.permissions, self.uid, self.gid);
            reply.attr(&Duration::ZERO, &attrs);
        } else {
            debug!("Failed to find entry");
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
        info!("readdir: {ino} {fh} {offset}");
        if !self.handles.contains_key(&ino) {
            reply.error(ENOENT);
            return;
        }
        if let Some(FSEntry::Dir { children, .. }) = self.inode_map.get(&ino) {
            for (i, (name, id)) in children.iter().enumerate().skip(offset as usize) {
                let child = self.inode_map.get(id).unwrap();
                debug!("adding {}", name);
                let full = reply.add(*id, i as i64 + 1, child.kind(), name);
                if full {
                    debug!("readdir full");
                    break;
                }
            }
            reply.ok()
        } else {
            reply.error(ENOENT)
        }
    }

    fn open(&mut self, _req: &fuser::Request<'_>, ino: u64, flags: i32, reply: fuser::ReplyOpen) {
        info!("open: {ino} {flags}");
        if self.inode_map.contains_key(&ino) {
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
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: fuser::ReplyData,
    ) {
        info!("read: {ino} {fh} {offset} {size}");
        if let Some(FSEntry::File { content, .. }) = self.inode_map.get(&ino) {
            reply.data(content.as_bytes());
        } else {
            reply.error(ENOENT);
        }
    }
}

fn sanitize_name(name: &str) -> String {
    pub const PROHIBITED_PATH_CHARS: &[char] =
        &['/', '\\', '?', '%', '*', ':', '|', '"', '<', '>', '.'];
    name.replace(PROHIBITED_PATH_CHARS, "")
}

fn filter_folders(folder_ids: Vec<Uuid>, secrets: &mut Vec<Secret>) {
    secrets.retain(|s| folder_ids.contains(&s.folder_id.unwrap_or_default()))
}
