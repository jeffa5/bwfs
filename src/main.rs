use std::process::{Command, Stdio};

use clap::Parser;
use fuser::Filesystem;

#[derive(Debug, Parser)]
struct Args {
    /// Where to mount the secrets.
    #[clap()]
    mountpoint: String,
}

struct NullFS;

impl Filesystem for NullFS {}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Args::parse();
    println!("{:?}", args);

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
    for secret in secrets {
        println!("{}", secret.name);
    }

    // println!("Configuring mount");
    // fuser::mount2(NullFS, args.mountpoint, &[]).unwrap();
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
