use std::{
    fs::remove_file,
    io::{BufRead, BufReader, ErrorKind, Write},
    os::unix::net::{UnixListener, UnixStream},
    sync::{mpsc, Arc, Mutex},
};

use clap::Args;
use fuser::MountOption;
use std::time::Duration;
use std::time::Instant;
use sysinfo::{Groups, Pid, Users};
use tracing::{debug, info, warn};

use bwclient::BWCLI;
use mapfs::MapFS;

use crate::{
    message::{Request, Response},
    server::bwclient::StatusKind,
};

use self::mapfs::MapFSRef;

pub mod bwclient;
pub mod mapfs;

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

    /// Lock the filesystem after the given number of seconds since unlock.
    ///
    /// Set to 0 to disable auto lock.
    #[clap(long, default_value = "300")]
    lock_after_s: u64,
}

pub fn serve(socket: String, args: ServeArgs) -> anyhow::Result<()> {
    let (fs, cli) = bw_init(&args);
    let fs_ref = MapFSRef(Arc::new(Mutex::new(fs)));
    let cli_ref = Arc::new(Mutex::new(cli));
    info!(args.mountpoint, "Configuring mount");
    let mut mount_options = Vec::new();
    mount_options.push(MountOption::RO);
    if !args.no_auto_unmount {
        mount_options.push(MountOption::AutoUnmount);
        mount_options.push(MountOption::AllowOther);
    }

    let (sender, receiver) = mpsc::channel::<()>();

    if args.lock_after_s > 0 {
        let fs = fs_ref.clone();
        let cli = Arc::clone(&cli_ref);
        std::thread::Builder::new()
            .name("lock-after".to_owned())
            .spawn(move || {
                debug!(args.lock_after_s, "Spawned lock-after thread");
                loop {
                    debug!("Waiting for unlock condition");
                    match receiver.recv() {
                        Ok(()) => {
                            debug!("Received unlock signal");
                            while let Ok(()) = receiver.try_recv() {
                                // just draining the queue so we don't get caught behind
                            }
                        }
                        Err(error) => {
                            warn!(%error, "Unlock channel failed to receive");
                            break;
                        }
                    }

                    let unlocked = cli
                        .lock()
                        .unwrap()
                        .status()
                        .map_or(false, |s| s.status == StatusKind::Unlocked);
                    if unlocked {
                        debug!(
                            args.lock_after_s,
                            "CLI unlocked, waiting for lock after duration"
                        );
                        std::thread::sleep(Duration::from_secs(args.lock_after_s));
                        debug!("Lock after duration passed, clearing and locking");
                        fs.clear();
                        cli.lock().unwrap().lock();
                    }
                }
            })
            .unwrap();
    }

    println!("Mount configured at {:?}", args.mountpoint);
    let _mount = fuser::spawn_mount2(fs_ref.clone(), args.mountpoint, &mount_options).unwrap();
    serve_commands(socket.clone(), &cli_ref, fs_ref, &sender);
    remove_file(socket)?;
    Ok(())
}

fn bw_init(args: &ServeArgs) -> (MapFS, BWCLI) {
    let uid = if let Some(user) = &args.user {
        let users = Users::new_with_refreshed_list();
        if let Some(user) = users.iter().find(|u| u.name() == user).map(|u| u.id()) {
            **user
        } else {
            panic!("Couldn't find user {user}");
        }
    } else {
        let s = sysinfo::System::new_all();
        let self_pid = std::process::id();
        *s.process(Pid::from_u32(self_pid))
            .unwrap()
            .user_id()
            .unwrap()
            .clone()
    };
    let gid = if let Some(group) = &args.group {
        let groups = Groups::new_with_refreshed_list();
        if let Some(group) = groups.iter().find(|g| g.name() == group).map(|g| g.id()) {
            **group
        } else {
            panic!("Couldn't find group {group}");
        }
    } else {
        let s = sysinfo::System::new_all();
        let self_pid = std::process::id();
        *s.process(Pid::from_u32(self_pid))
            .unwrap()
            .group_id()
            .unwrap()
    };
    let mode = u16::from_str_radix(&args.mode, 8).unwrap();
    debug!(
        uid,
        gid, mode, "Initialised bitwarden client and filesystem"
    );

    let fs = MapFS::new(uid, gid, mode, args.folders.clone());

    let cli = BWCLI::new(args.bw_bin.clone());
    (fs, cli)
}

fn serve_commands(
    socket: String,
    cli: &Arc<Mutex<BWCLI>>,
    fs: MapFSRef,
    unlock_notify: &mpsc::Sender<()>,
) {
    info!(socket, "Starting listening");
    let listener = bind_socket_or_remove(socket).unwrap();
    loop {
        let (stream, _addr) = listener.accept().unwrap();
        debug!("Accepted connection");
        handle_stream(stream, cli, fs.clone(), unlock_notify);
    }
}

fn bind_socket_or_remove(socket: String) -> anyhow::Result<UnixListener> {
    match UnixListener::bind(&socket) {
        Ok(l) => Ok(l),
        Err(e) => {
            if e.kind() == ErrorKind::AddrInUse {
                // try to connect to the socket
                if UnixStream::connect(&socket).is_ok() {
                    // managed to connect so probably something running, don't disturb it
                    // ourselves
                    debug!(socket, "Found socket file already in use and alive");
                    Err(anyhow::anyhow!(
                        "Socket {socket} exists and is already listening from another instance"
                    ))
                } else {
                    debug!(
                        socket,
                        "Found socket file already in but dead, deleting it and binding"
                    );
                    // nothing running, we can semi-safely remove the socket
                    remove_file(&socket)?;
                    let l = UnixListener::bind(socket)?;
                    Ok(l)
                }
            } else {
                Err(anyhow::Error::from(e))
            }
        }
    }
}

fn handle_stream(
    stream: UnixStream,
    cli: &Arc<Mutex<BWCLI>>,
    fs: MapFSRef,
    unlock_notify: &mpsc::Sender<()>,
) {
    let mut input = Vec::new();
    let mut reader = BufReader::new(stream);
    reader.read_until(b'\n', &mut input).unwrap();
    let mut stream = reader.into_inner();
    // stream.read_to_string(&mut input).unwrap();
    // debug!(input, "Got input");
    match serde_json::from_slice::<Request>(&input) {
        Ok(request) => {
            debug!("Parsed request");
            let res = handle_request(request, cli, fs, unlock_notify);
            debug!(?res, "Sending response");
            let json_res = serde_json::to_vec(&res).unwrap();
            stream.write_all(&json_res).unwrap();
        }
        Err(e) => {
            warn!(error=%e, "Failed to parse client request");
        }
    }
}

fn handle_request(
    request: Request,
    cli: &Arc<Mutex<BWCLI>>,
    fs: MapFSRef,
    unlock_notify: &mpsc::Sender<()>,
) -> Response {
    match request {
        Request::Unlock { password } => {
            let start = Instant::now();
            let res = match cli.lock().unwrap().unlock(&password) {
                Ok(()) => {
                    let _ = unlock_notify.send(());
                    Response::Success
                }
                Err(e) => Response::Failure {
                    reason: e.to_string(),
                },
            };
            let time = start.elapsed();
            debug!(?time, "Unlock complete");
            res
        }
        Request::Lock => {
            let start = Instant::now();
            fs.clear();
            cli.lock().unwrap().lock();
            let time = start.elapsed();
            debug!(?time, "Lock complete");
            Response::Success
        }
        Request::Status => {
            let start = Instant::now();
            let res = match cli.lock().unwrap().status() {
                Ok(s) => Response::Status {
                    locked: s.status == StatusKind::Locked,
                },
                Err(e) => Response::Failure {
                    reason: e.to_string(),
                },
            };
            let time = start.elapsed();
            debug!(?time, "Status complete");
            res
        }
        Request::Refresh => {
            let start = Instant::now();
            let res = match fs.refresh(&cli.lock().unwrap()) {
                Ok(()) => Response::Success,
                Err(e) => Response::Failure {
                    reason: e.to_string(),
                },
            };
            let time = start.elapsed();
            debug!(?time, "Refresh complete");
            res
        }
    }
}
