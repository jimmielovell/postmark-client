mod attachment;
mod email;
mod error;

use std::time::Duration;

use crate::attachment::Attachment;
use crate::error::ClientError;
use email::Email;
use reqwest::Url;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Debug)]
pub struct Client {
    http_client: reqwest::Client,
    base_url: Url,
    sender: Email,
    auth_token: SecretString,
    timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct ClientBuilder {
    base_url: Option<Url>,
    sender: Option<Email>,
    auth_token: Option<SecretString>,
    timeout: Option<Duration>,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            base_url: None,
            sender: None,
            auth_token: None,
            timeout: Some(DEFAULT_TIMEOUT),
        }
    }
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
        skip(
            self,
            recipient,
            subject,
            html_content,
            text_content,
            name,
            attachments
        )
    )]
    pub async fn send(
        &self,
        recipient: &Email,
        subject: &str,
        html_content: &str,
        text_content: &str,
        name: Option<&str>,
        attachments: Option<Vec<Attachment>>,
    ) -> Result<SendEmailResponse, ClientError> {
        let url = self
            .base_url
            .join("/email")
            .map_err(|e| ClientError::Configuration(format!("Postmark invalid URL: {}", e)))?;

        let to = match name {
            Some(name) => format!("{} <{}>", name, recipient.as_ref()),
            None => recipient.as_ref().to_owned(),
        };

        let body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: to.as_str(),
            subject,
            tag: None,
            html_body: html_content,
            text_body: text_content,
            metadata: None,
            track_opens: true,
            track_links: "HtmlAndText",
            attachments,
        };

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
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    tag: Option<&'a str>,
    html_body: &'a str,
    text_body: &'a str,
    metadata: Option<serde_json::Value>,
    track_opens: bool,
    track_links: &'a str,
    attachments: Option<Vec<Attachment>>,
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
    use crate::email::Email;
    use crate::{Client, SendEmailResponse};
    use claim::{assert_err, assert_ok};
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::Fake;
    use reqwest::Url;
    use secrecy::SecretString;
    use wiremock::matchers::{any, header, header_exists, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    /// Generate a random email subject
    fn subject() -> String {
        Sentence(1..2).fake()
    }

    /// Generate a random email content
    fn content() -> String {
        Paragraph(1..10).fake()
    }

    /// Generate a random subscriber email
    fn email() -> Email {
        Email::parse(SafeEmail().fake::<String>().as_str()).unwrap()
    }

    /// Get a test instance of `EmailClient`.
    fn email_client(base_url: &str) -> Client {
        let base_url = Url::parse(base_url).expect("Failed to parse base uri");
        let auth_token = 13.fake::<String>();
        let auth_token = SecretString::from(auth_token);

        Client::builder()
            .base_url(base_url)
            .sender(email())
            .auth_token(auth_token)
            .timeout(std::time::Duration::from_secs(1))
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn send_email_sends_expected_request() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(&mock_server.uri());

        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let _ = email_client
            .send(&email(), &subject(), &content(), &content(), None, None)
            .await;
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_200() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(&mock_server.uri());

        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200).set_body_json(SendEmailResponse::default()))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send(&email(), &subject(), &content(), &content(), None, None)
            .await;

        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(&mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send(&email(), &subject(), &content(), &content(), None, None)
            .await;

        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(&mock_server.uri());

        let response = ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(180));

        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;
        let outcome = email_client
            .send(&email(), &subject(), &content(), &content(), None, None)
            .await;

        assert_err!(outcome);
    }

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            // Try to parse the body as a JSON value
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);
            if let Ok(body) = result {
                // Check that all the mandatory fields are populated
                body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some()
            } else {
                // If parsing failed, do not match the request
                false
            }
        }
    }
}
