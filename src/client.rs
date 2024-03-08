use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
};

use anyhow::Context;
use tracing::debug;

use crate::message::{Request, Response};

pub fn unlock(socket: String, no_refresh: bool) -> anyhow::Result<()> {
    let password = rpassword::prompt_password("Bitwarden password (input is hidden): ").unwrap();
    let request = Request::Unlock { password };
    match send_msg(socket.clone(), request)? {
        Response::Success => println!("Unlocked"),
        Response::Failure => println!("Failed to unlock"),
        _ => unreachable!(),
    }
    if !no_refresh {
        match send_msg(socket, Request::Refresh)? {
            Response::Success => println!("Refreshed"),
            Response::Failure => println!("Failed to refresh"),
            _ => unreachable!(),
        }
    }
    Ok(())
}

pub fn lock(socket: String) -> anyhow::Result<()> {
    let request = Request::Lock;
    match send_msg(socket.clone(), request)? {
        Response::Success => println!("Locked"),
        Response::Failure => println!("Failed to lock"),
        _ => unreachable!(),
    }
    match send_msg(socket, Request::Refresh)? {
        Response::Success => println!("Refreshed"),
        Response::Failure => println!("Failed to refresh"),
        _ => unreachable!(),
    }
    Ok(())
}

pub fn status(socket: String) -> anyhow::Result<()> {
    let request = Request::Status;
    match send_msg(socket, request)? {
        Response::Status { locked } => {
            if locked {
                println!("Locked");
            } else {
                println!("Unlocked");
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}

pub fn refresh(socket: String) -> anyhow::Result<()> {
    match send_msg(socket, Request::Refresh)? {
        Response::Success => println!("Refreshed"),
        Response::Failure => println!("Failed to refresh"),
        _ => unreachable!(),
    }
    Ok(())
}

fn send_msg(socket: String, request: Request) -> anyhow::Result<Response> {
    let mut stream = UnixStream::connect(&socket).context(socket.clone())?;
    debug!(socket, "Connected to server");
    let request_json = serde_json::to_vec(&request)?;
    stream.write_all(&request_json)?;
    stream.write_all(b"\n")?;
    debug!(socket, "Sent request");
    let mut response_json = String::new();
    stream.read_to_string(&mut response_json)?;
    debug!(socket, "Got response");
    let res = serde_json::from_str(&response_json)?;
    Ok(res)
}
