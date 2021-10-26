use anyhow::Error;
use reqwest::{Client, Body};
use tokio::io::{AsyncRead};
use std::time::Duration;
use serde_json::json;
use serde::{Serialize, Deserialize};
use tokio_util::io::ReaderStream;

use crate::oauth::OAuth;

#[derive(Debug, Clone)]
pub struct GDrive {
    oauth: OAuth,
    client: Client
}

impl GDrive {
    pub fn new(oauth: OAuth) -> GDrive {
        GDrive {
            oauth,
            client: Client::builder()
                .timeout(Duration::from_secs(16))
                .build()
                .unwrap()
        }
    }

    // Upload file
    pub async fn upload<'a>(
        &self, 
        input: impl AsyncRead + Send + Sync + 'static, 
        filename: &str, 
        parent: &str,
        // Update file rather than create new one
        file_id: Option<&str>
    ) -> Result<GDriveUploaded, Error> {
        // Get initial upload URL
        let req = match file_id {
            Some(file_id) => self.client.patch(&format!("https://www.googleapis.com/upload/drive/v3/files/{}", file_id)),
            None => self.client.post("https://www.googleapis.com/upload/drive/v3/files")
        };
        let res = req
            .bearer_auth(self.oauth.token().await?)
            .query(&[
                ("uploadType", "resumable"),
                ("supportsAllDrives", "true"),
            ]);
        let res = match file_id {
            Some(_) => res.json(&json!({
                "name": filename,
            })),
            None => res.json(&json!({
                "name": filename,
                "parents": [parent]
            }))
        }; 
        let res = res.send().await?.error_for_status()?;
        let url = res.headers().get("location").ok_or(anyhow!("Missing location header in response"))?.to_str()?;
        
        // Upload
        let stream = ReaderStream::new(input);
        let res = self.client.put(url)
            .body(Body::wrap_stream(stream))
            .send()
            .await?
            .error_for_status()?;

        let out: GDriveUploaded = res.json().await?;
        Ok(out)
    }

    /// Get children from folder
    pub async fn list_folder(&self, id: &str) -> Result<GDriveChildList, Error> {
        let r = self.client.get(&format!("https://www.googleapis.com/drive/v2/files/{}/children", id))
            .query(&[
                ("maxResults", 999),
            ])
            .bearer_auth(self.oauth.token().await?)
            .send().await?.error_for_status()?.json().await?;
        Ok(r)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GDriveFile {
    pub id: String,
    pub web_content_link: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GDriveUploaded {
    pub kind: String,
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub drive_id: Option<String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GDriveChildList {
    pub next_page_token: Option<String>,
    pub next_link: Option<String>,
    pub items: Vec<GDriveChild>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GDriveChild {
    pub id: String,
    pub self_link: String,
    pub child_link: String
}