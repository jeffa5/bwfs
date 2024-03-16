use std::{
    fmt::Display,
    process::{Command, Stdio},
};
use time::OffsetDateTime;
use tracing::{debug, info};
use uuid::Uuid;

pub struct BWCLI {
    path: String,
    session_token: Option<String>,
}

impl BWCLI {
    pub fn new(bin_path: String) -> Self {
        Self {
            path: bin_path,
            session_token: None,
        }
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new(&self.path);
        cmd.args(args);
        info!("Executing command {:?}", cmd);
        if let Some(session_token) = &self.session_token {
            debug!("Adding BW_SESSION env");
            cmd.env("BW_SESSION", session_token);
        }
        cmd
    }

    pub fn status(&self) -> anyhow::Result<Status> {
        let output = self.command(&["status"]).output()?;
        let stdout = String::from_utf8(output.stdout)?;
        let status: Status = serde_json::from_str(&stdout)?;
        debug!(?status, "Got status");
        Ok(status)
    }

    pub fn unlock(&mut self, password: &str) -> anyhow::Result<()> {
        const BWFS_PASSWORD: &str = "BWFS_PASSWORD";
        debug!("Unlocking vault");
        let output = self
            .command(&["unlock", "--raw", "--passwordenv", BWFS_PASSWORD])
            .env(BWFS_PASSWORD, password)
            .output()?;
        if output.status.success() {
            let session_token = String::from_utf8(output.stdout)?;
            debug!("Got session token");
            self.session_token = Some(session_token);
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                String::from_utf8(output.stderr).unwrap_or_default()
            ))
        }
    }

    pub fn lock(&mut self) -> anyhow::Result<()> {
        self.session_token = None;
        Ok(())
    }

    pub fn list_secrets(&self) -> anyhow::Result<Vec<Secret>> {
        let output = self.command(&["list", "items"]).output()?;
        let stdout = String::from_utf8(output.stdout)?;
        let secrets_list: Vec<Secret> = serde_json::from_str(&stdout)?;
        Ok(secrets_list)
    }

    pub fn list_folders(&self) -> anyhow::Result<Vec<Folder>> {
        let output = self.command(&["list", "folders"]).output()?;
        let stdout = String::from_utf8(output.stdout)?;
        let folders_list: Vec<Folder> = serde_json::from_str(&stdout)?;
        Ok(folders_list)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    #[serde(with = "time::serde::rfc3339")]
    pub last_sync: OffsetDateTime,
    pub user_email: String,
    pub user_id: Uuid,
    pub status: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Secret {
    pub password_history: Option<Vec<SecretPasswordHistory>>,
    #[serde(with = "time::serde::rfc3339")]
    pub revision_date: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub creation_date: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_date: Option<OffsetDateTime>,
    pub object: String,
    pub id: Uuid,
    pub organization_id: Option<Uuid>,
    pub folder_id: Option<Uuid>,
    pub r#type: SecretType,
    pub reprompt: u32,
    pub name: String,
    pub notes: Option<String>,
    pub favorite: bool,
    pub fields: Option<Vec<SecretField>>,
    pub login: Option<SecretLogin>,
    pub collection_ids: Vec<Uuid>,
}

#[derive(Debug, serde_repr::Serialize_repr, serde_repr::Deserialize_repr)]
#[repr(u8)]
pub enum SecretType {
    Login = 1,
    Card = 2,
    Identity = 3,
    SecureNote = 4,
}

impl Display for SecretType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SecretType::Login => "Login",
            SecretType::Card => "Card",
            SecretType::Identity => "Identity",
            SecretType::SecureNote => "Secure note",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretLogin {
    pub fido_2_credentials: Vec<String>,
    pub uris: Option<Vec<SecretLoginUri>>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub totp: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub password_revision_date: Option<OffsetDateTime>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretLoginUri {
    pub r#match: Option<String>,
    pub uri: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretPasswordHistory {
    #[serde(with = "time::serde::rfc3339")]
    pub last_used_date: OffsetDateTime,
    pub password: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretField {
    pub name: String,
    pub value: String,
    pub r#type: SecretFieldType,
}

#[derive(Debug, serde_repr::Serialize_repr, serde_repr::Deserialize_repr)]
#[repr(u8)]
pub enum SecretFieldType {
    Text = 0,
    Hidden = 1,
    Boolean = 2,
    Linked = 3,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub object: String,
    pub id: Option<Uuid>,
    pub name: String,
}
