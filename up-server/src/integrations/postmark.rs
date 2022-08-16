use chrono::{DateTime, Utc};
use miette::Diagnostic;
use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use tracing::Level;

use crate::mask;

pub type Result<T> = miette::Result<T, PostmarkError>;

const POSTMARK_API_TOKEN_ENV: &str = "POSTMARK_API_TOKEN";
const POSTMARK_API_BASE_URL: &str = "https://api.postmarkapp.com";
const POSTMARK_API_ENDPOINT_EMAIL: &str = "/email";
const POSTMARK_API_TOKEN_HEADER: &str = "X-Postmark-Server-Token";
const POSTMARK_API_TEST_TOKEN: &str = "POSTMARK_API_TEST";

#[derive(Clone)]
pub struct PostmarkClient {
    token: String,
    api_base_url: url::Url,
    client: reqwest::Client,
}

#[derive(Error, Diagnostic, Debug)]
pub enum PostmarkError {
    #[error("expected Postmark token in POSTMARK_API_TOKEN environment variable")]
    #[diagnostic(code(up::config::invalid))]
    MissingToken,
    #[error("failed to create HTTP client: {0}")]
    ClientBuildError(reqwest::Error),
    #[error("failed to create HTTP request: {0}")]
    RequestBuildError(reqwest::Error),
    #[error("failed to execute HTTP request: {0}")]
    RequestError(reqwest::Error),
    #[error("failed to parse API response: {0}")]
    ResponseParseError(serde_json::Error),
    #[error("failed to parse API URL: {0}")]
    UrlParsingError(url::ParseError),
    #[error("failed to send email using Postmark: {1} ({0})")]
    ApiError(i32, String),
    #[error("HTTP error sending email using Postmark: {1} ({0})")]
    ApiHttpError(StatusCode, String),
}

impl PostmarkClient {
    pub fn new() -> Result<Self> {
        let token =
            std::env::var(POSTMARK_API_TOKEN_ENV).map_err(|_| PostmarkError::MissingToken)?;
        let client = reqwest::Client::builder()
            .build()
            .map_err(PostmarkError::ClientBuildError)?;
        let api_base_url: url::Url = POSTMARK_API_BASE_URL
            .parse()
            .map_err(PostmarkError::UrlParsingError)?;

        if token == POSTMARK_API_TEST_TOKEN {
            tracing::warn!(
                "the Postmark token is the API test token, emails will not actually be sent"
            );
        }

        Ok(Self {
            token,
            api_base_url,
            client,
        })
    }

    pub async fn send_email(&self, request: &SendEmailRequest) -> Result<()> {
        let req = self
            .client
            .request(
                Method::POST,
                self.api_base_url
                    .join(POSTMARK_API_ENDPOINT_EMAIL)
                    .map_err(PostmarkError::UrlParsingError)?,
            )
            .header(POSTMARK_API_TOKEN_HEADER, &self.token)
            .json(request)
            .build()
            .map_err(PostmarkError::RequestBuildError)?;

        let resp = self
            .client
            .execute(req)
            .await
            .map_err(PostmarkError::RequestError)?;

        let status = resp.status();

        let response_body_bytes = resp.bytes().await.map_err(PostmarkError::RequestError)?;

        if !status.is_success() {
            return Err(PostmarkError::ApiHttpError(
                status,
                String::from_utf8_lossy(&response_body_bytes).to_string(),
            )
            .into());
        }

        let api_response: SendEmailResponse = serde_json::from_slice(&response_body_bytes)
            .map_err(PostmarkError::ResponseParseError)?;

        if api_response.error_code == 0 {
            if tracing::event_enabled!(Level::TRACE) {
                let emails = request
                    .to
                    .split(',')
                    .into_iter()
                    .map(|e| mask::email(e.trim()))
                    .collect::<Vec<_>>()
                    .join(", ");
                let subject = request.subject.as_deref().unwrap_or("");
                tracing::info!(emails = emails, subject = subject, "emails sent");
            }
            Ok(())
        } else {
            Err(PostmarkError::ApiError(api_response.error_code, api_response.message).into())
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct SendEmailRequest {
    pub from: String,
    pub to: String,
    #[serde(flatten)]
    pub body: Body,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SendEmailResponse {
    pub error_code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submitted_at: Option<DateTime<Utc>>,
    pub to: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Body {
    #[serde(rename = "TextBody")]
    Text(String),
    #[serde(rename = "HtmlBody")]
    Html(String),
}

impl Default for Body {
    fn default() -> Self {
        Body::Text("".into())
    }
}
