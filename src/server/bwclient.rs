use std::{
    fmt::Display,
    process::{Command, Stdio},
};
use time::OffsetDateTime;
use tracing::info;
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

    pub fn unlock(&mut self, password: &str) -> Result<(), String> {
        const BWFS_PASSWORD: &str = "BWFS_PASSWORD";
        let output = self
            .command(&["unlock", "--raw", "--passwordenv", BWFS_PASSWORD])
            .env(BWFS_PASSWORD, password)
            .stdin(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| e.to_string())?;
        let session_token = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
        self.session_token = Some(session_token);
        Ok(())
    }

    pub fn lock(&mut self) -> Result<(), String> {
        self.command(&["lock"])
            .stdin(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_secrets(&self) -> Result<Vec<Secret>, String> {
        let output = self
            .command(&["list", "items"])
            .output()
            .map_err(|e| e.to_string())?;
        let stdout = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
        let secrets_list: Vec<Secret> = serde_json::from_str(&stdout).map_err(|e| e.to_string())?;
        Ok(secrets_list)
    }

    pub fn list_folders(&self) -> Result<Vec<Folder>, String> {
        let output = self
            .command(&["list", "folders"])
            .output()
            .map_err(|e| e.to_string())?;
        let stdout = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
        let folders_list: Vec<Folder> = serde_json::from_str(&stdout).map_err(|e| e.to_string())?;
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
