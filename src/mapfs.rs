use fuser::FileAttr;
use fuser::FileType;
use fuser::Filesystem;
use libc::ENOENT;
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime};
use tracing::debug;
use tracing::info;

#[derive(Debug)]
pub enum FSEntry {
    Dir(BTreeMap<String, u64>),
    File {
        content: String,
        ctime: SystemTime,
        mtime: SystemTime,
    },
}

impl FSEntry {
    fn attrs(&self, ino: u64) -> FileAttr {
        FileAttr {
            ino,
            size: self.size(),
            blocks: 1,
            atime: SystemTime::now(),
            mtime: self.mtime(),
            ctime: self.ctime(),
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
            FSEntry::File { .. } => FileType::RegularFile,
        }
    }

    fn size(&self) -> u64 {
        match self {
            FSEntry::Dir(_) => 0,
            FSEntry::File { content, .. } => content.as_bytes().len() as u64,
        }
    }

    fn ctime(&self) -> SystemTime {
        match self {
            FSEntry::Dir(_) => SystemTime::now(),
            FSEntry::File { ctime, .. } => ctime.clone(),
        }
    }

    fn mtime(&self) -> SystemTime {
        match self {
            FSEntry::Dir(_) => SystemTime::now(),
            FSEntry::File { mtime, .. } => mtime.clone(),
        }
    }
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

    pub fn add_dir(&mut self, parent: u64, name: String) -> u64 {
        let name = sanitize_name(&name);
        let inode = self.next_id();
        if let Some(FSEntry::Dir(children)) = self.inode_map.get_mut(&parent) {
            children.insert(name.clone(), inode);
        }
        self.name_map.insert((parent, name), inode);
        self.inode_map.insert(inode, FSEntry::Dir(BTreeMap::new()));
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
        if let Some(FSEntry::Dir(children)) = self.inode_map.get_mut(&parent) {
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
        if let Some(ino) = self.name_map.get(&(parent, name.to_owned())).copied() {
            let entry = self.inode_map.get(&ino).unwrap();
            debug!("looked up secret {}", name);
            let attrs = entry.attrs(ino);
            reply.entry(&Duration::from_secs(60), &attrs, self.generation)
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
            let attrs = entry.attrs(ino);
            reply.attr(&Duration::from_secs(60), &attrs);
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
        if let Some(FSEntry::Dir(children)) = self.inode_map.get(&ino) {
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
