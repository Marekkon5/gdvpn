use anyhow::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json::{json, Value};
use serde::{Serialize, Deserialize};
use dialoguer::{theme::ColorfulTheme, Input};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use reqwest::Client;

#[derive(Debug, Clone)]
pub struct OAuth {
    app: OAuthAppInfo,
    token: Arc<Mutex<OAuthTokenInfo>>,
    expires: Arc<Mutex<u128>>,
    client: Client
}

impl OAuth {
    pub async fn new(app_info: &str, user_token: Option<&str>, scope: Option<&str>) -> Result<OAuth, Error> {
        let client = Client::new();
        let app = OAuthAppInfo::load(app_info).await?;
        // Has token
        if let Some(token_path) = user_token {
            if let Ok(token) = OAuthTokenInfo::load(token_path).await {
                let oauth = OAuth {
                    client, app, 
                    token: Arc::new(Mutex::new(token)), 
                    expires: Arc::new(Mutex::new(0))
                };
                oauth.refresh_token().await?;
                return Ok(oauth);
            }
        }

        // Generate URL
        let code_verifier = pkce::code_verifier(64);
        let code_challenge = pkce::code_challenge(&code_verifier);
        let default_redirect_uri = "urn:ietf:wg:oauth:2.0:oob".to_string();
        let redirect_uri = app.redirect_uris.first().unwrap_or(&default_redirect_uri);
        let url = format!(
            "{}?code_challenge_method=S256&scope={}&access_type=offline&response_type=code&client_id={}&redirect_uri={}&code_challenge={}&state={}",
            &app.auth_uri,
            urlencoding::encode(scope.ok_or(anyhow!("Missing scope!"))?),
            urlencoding::encode(&app.client_id),
            urlencoding::encode(&redirect_uri),
            code_challenge,
            "state"
        );
        println!("Go to this URL: {}", url);
        // Get user response
        let response_code: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter returned code")
            .interact_text()?;
        
        // Get token
        let token: OAuthTokenInfo = client.post(&app.token_uri)
            .form(&json!({
                "code": response_code,
                "client_id": &app.client_id,
                "client_secret": &app.client_secret,
                "redirect_uri": redirect_uri,
                "grant_type": "authorization_code",
                "code_verifier": String::from_utf8(code_verifier)?
            }))
            .send()
            .await?
            .json()
            .await?;

        // Save
        if let Some(path) = user_token {
            token.save(path).await?;
        }
        let expires = timestamp!() + ((token.expires_in - 10) * 1000) as u128;

        Ok(OAuth {
            client, 
            app,
            token: Arc::new(Mutex::new(token)),
            expires: Arc::new(Mutex::new(expires))
        })
    }

    // Refresh authentication token using refresh token
    async fn refresh_token(&self) -> Result<(), Error> {
        let mut token = self.token.lock().await;
        let response: OAuthTokenInfo = self.client.post(&self.app.token_uri)
            .form(&json!({
                "client_id": self.app.client_id,
                "client_secret": self.app.client_secret,
                "grant_type": "refresh_token",
                "refresh_token": token.refresh_token.as_ref().ok_or(anyhow!("No refresh URL!"))?, 
            }))
            .send()
            .await?
            .json()
            .await?;
        token.access_token = response.access_token;
        *(self.expires.lock().await) = timestamp!() + ((response.expires_in - 10) * 1000) as u128;
        Ok(())
    }

    // Get token, if expired refresh
    pub async fn token(&self) -> Result<String, Error> {
        if timestamp!() > *(self.expires.lock().await) {
            self.refresh_token().await?;
        }
        Ok(self.token.lock().await.access_token.to_string())
    }

}

// Wrapped inside "installed", can be downloaded from google console
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OAuthAppInfo {
    pub client_id: String,
    pub auth_uri: String,
    pub token_uri: String,
    pub client_secret: String,
    pub redirect_uris: Vec<String>
}

impl OAuthAppInfo {
    pub async fn load(path: &str) -> Result<OAuthAppInfo, Error> {
        let mut file = File::open(path).await?;
        let mut data = String::new();
        file.read_to_string(&mut data).await?;
        let json: Value = serde_json::from_str(&data)?;
        Ok(serde_json::from_value(json["installed"].clone())?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OAuthTokenInfo {
    pub access_token: String,
    pub expires_in: i64,
    pub refresh_token: Option<String>,
    pub scope: String,
    pub token_type: String
}

impl OAuthTokenInfo {
    pub async fn load(path: &str) -> Result<OAuthTokenInfo, Error> {
        let mut file = File::open(path).await?;
        let mut data = String::new();
        file.read_to_string(&mut data).await?;
        Ok(serde_json::from_str(&data)?)
    }

    pub async fn save(&self, path: &str) -> Result<(), Error> {
        let mut file = File::create(path).await?;
        file.write(serde_json::to_string(self)?.as_bytes()).await?;
        Ok(())
    }
}