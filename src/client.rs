use std::process::{Command, Stdio};
use tracing::info;

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
    pub last_sync: String,
    pub user_email: String,
    pub user_id: String,
    pub status: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Secret {
    pub password_history: Option<Vec<SecretPasswordHistory>>,
    pub revision_date: String,
    pub creation_date: String,
    pub deleted_date: Option<String>,
    pub object: String,
    pub id: String,
    pub organization_id: Option<String>,
    pub folder_id: Option<String>,
    pub r#type: u32,
    pub reprompt: u32,
    pub name: String,
    pub notes: Option<String>,
    pub favorite: bool,
    pub login: Option<SecretLogin>,
    pub collection_ids: Vec<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretLogin {
    pub fido_2_credentials: Vec<String>,
    pub uris: Option<Vec<SecretLoginUri>>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub totp: Option<String>,
    pub password_revision_date: Option<String>,
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
    pub last_used_date: String,
    pub password: String,
}
