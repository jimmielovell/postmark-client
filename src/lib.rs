mod attachment;

mod email;
pub use email::Email;

mod outbound_email_body;
pub use outbound_email_body::*;

mod error;

use std::time::Duration;

pub use crate::attachment::Attachment;
pub use crate::error::{ClientError, ParseError};
pub use reqwest::Url;
pub use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_BATCH_SIZE: usize = 500;

#[derive(Clone, Debug)]
pub struct Client {
    http_client: reqwest::Client,
    base_url: Url,
    sender: Email,
    auth_token: SecretString,
    timeout: Duration,
}

#[derive(Debug, Clone, Default)]
pub struct ClientBuilder {
    base_url: Option<Url>,
    sender: Option<Email>,
    auth_token: Option<SecretString>,
    timeout: Option<Duration>,
}

impl ClientBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn base_url(mut self, url: Url) -> Self {
        self.base_url = Some(url);
        self
    }

    pub fn sender(mut self, sender: Email) -> Self {
        self.sender = Some(sender);
        self
    }

    pub fn auth_token(mut self, token: SecretString) -> Self {
        self.auth_token = Some(token);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn build(self) -> Result<Client, ClientError> {
        let base_url = self.base_url.ok_or_else(|| {
            ClientError::Configuration("Postmark base URL is required".to_string())
        })?;
        let sender = self.sender.ok_or_else(|| {
            ClientError::Configuration("Postmark sender email is required".to_string())
        })?;
        let auth_token = self.auth_token.ok_or_else(|| {
            ClientError::Configuration("Postmark auth token is required".to_string())
        })?;

        let timeout = self.timeout.unwrap_or(DEFAULT_TIMEOUT);

        let http_client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(ClientError::Reqwest)?;

        Ok(Client {
            http_client,
            base_url,
            sender,
            auth_token,
            timeout,
        })
    }
}

impl Client {
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    #[tracing::instrument(
        name = "Sending email using email(postmark) client",
        skip(self, body)
    )]
    pub async fn send(
        &self,
        body: &OutboundEmailBody,
    ) -> Result<SendEmailResponse, ClientError> {
        let url = self
            .base_url
            .join("/email")
            .map_err(|e| ClientError::Configuration(format!("Postmark invalid URL: {}", e)))?;

        let body: SendEmailRequest = (body, &self.sender).into();

        let resp = self
            .http_client
            .post(url)
            .header("X-Postmark-Server-Token", self.auth_token.expose_secret())
            .json(&body)
            .send()
            .await
            .map_err(|err| {
                tracing::error!("Postmark: failed to send email: {}", err);
                if err.is_timeout() {
                    ClientError::Timeout(self.timeout.as_secs())
                } else {
                    ClientError::Reqwest(err)
                }
            })?;

        let status_code = resp.status();
        let message = resp.text().await.map_err(|err| {
            tracing::error!("Postmark: failed to read response body: {}", err);
            ClientError::Reqwest(err)
        })?;

        if status_code.is_success() {
            serde_json::from_str(&message).map_err(|err| {
                tracing::error!("Postmark: failed to parse response: {}", err);
                ClientError::Serde(err)
            })
        } else if status_code.as_str() == "401" {
            Err(ClientError::Authentication(message))
        } else {
            Err(ClientError::ServerResponse {
                status_code,
                message,
            })
        }
    }

    #[tracing::instrument(
        name = "Sending batch emails using email(postmark) client",
        skip(self, bodies)
    )]
    pub async fn send_batch(
        &self,
        bodies: &[OutboundEmailBody],
    ) -> Result<Vec<SendEmailResponse>, ClientError> {
        if bodies.is_empty() {
            return Ok(vec![]);
        }

        if bodies.len() > MAX_BATCH_SIZE {
            return Err(ClientError::Configuration(format!(
                "Batch size exceeds maximum allowed ({MAX_BATCH_SIZE})"
            )));
        }

        let url = self
            .base_url
            .join("/email/batch")
            .map_err(|e| ClientError::Configuration(format!("Invalid batch URL: {}", e)))?;

        let body: Vec<SendEmailRequest> = bodies
            .iter()
            .map(|body| (body, &self.sender).into())
            .collect();

        let resp = self
            .http_client
            .post(url.clone())
            .header("X-Postmark-Server-Token", self.auth_token.expose_secret())
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to send batch email: {}", e);
                ClientError::Reqwest(e)
            })?;

        let status_code = resp.status();
        let message = resp.text().await.map_err(|err| {
            tracing::error!("Postmark: failed to read batch response body: {}", err);
            ClientError::Reqwest(err)
        })?;

        if status_code.is_success() {
            serde_json::from_str(&message).map_err(|err| {
                tracing::error!("Postmark: failed to parse batch response: {}", err);
                ClientError::Serde(err)
            })
        } else if status_code.as_str() == "401" {
            Err(ClientError::Authentication(message))
        } else {
            Err(ClientError::ServerResponse {
                status_code,
                message,
            })
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    cc: Option<Vec<&'a str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bcc: Option<Vec<&'a str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subject: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tag: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    html_body: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text_body: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<Value>,
    track_opens: bool,
    track_links: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    attachments: Option<Vec<Attachment>>,
}

impl<'a> From<(&'a OutboundEmailBody, &'a Email)> for SendEmailRequest<'a> {
    fn from((request, from): (&'a OutboundEmailBody, &'a Email)) -> Self {
        SendEmailRequest {
            from: from.as_ref(),
            to: request.to.as_ref(),
            cc: request
                .cc
                .as_ref()
                .map(|emails| emails.iter().map(|email| email.as_ref()).collect()),
            bcc: request
                .bcc
                .as_ref()
                .map(|emails| emails.iter().map(|email| email.as_ref()).collect()),
            subject: request.subject.as_deref(),
            tag: request.tag.as_deref(),
            html_body: request.html_body.as_deref(),
            text_body: request.text_body.as_deref(),
            reply_to: request.reply_to.as_ref().map(|reply_to| reply_to.as_ref()),
            metadata: request.metadata.clone(),
            track_opens: request.track_opens,
            track_links: request.track_links.as_str(),
            attachments: request.attachments.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SendEmailResponse {
    error_code: i16,
    message: String,
    #[serde(rename = "MessageID")]
    message_id: String,
    submitted_at: String,
    to: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_internal_request_conversion() {
        let to = Email::parse("recipient@example.com").unwrap();
        let cc_email = Email::parse("cc@example.com").unwrap();

        let request = OutboundEmailBody::builder(to)
            .subject("Test Subject")
            .html_body("<p>HTML Content</p>")
            .text_body("Text Content")
            .cc(vec![cc_email])
            .build();

        let from = Email::parse("from@example.com").unwrap();
        let internal: SendEmailRequest = (&request, &from).into();

        assert_eq!(internal.from, "from@example.com");
        assert_eq!(internal.to, "recipient@example.com");
        assert_eq!(internal.cc.unwrap()[0], "cc@example.com");
        assert_eq!(internal.subject, Some("Test Subject"));
        assert_eq!(internal.html_body, Some("<p>HTML Content</p>"));
        assert_eq!(internal.text_body, Some("Text Content"));
        assert!(internal.track_opens);
        assert_eq!(internal.track_links, "HtmlAndText");
    }
}
