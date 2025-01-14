#[cfg(test)]
mod tests {
    use claim::{assert_err, assert_ok};
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Sentence};
    use fake::Fake;
    use postmark_client::{Client, Email, OutboundEmailBody, SendEmailResponse};
    use reqwest::Url;
    use secrecy::SecretString;
    use wiremock::matchers::{any, header, header_exists, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    fn build_outbound_email_body() -> OutboundEmailBody{
        let to = Email::parse(SafeEmail().fake::<String>().as_str()).unwrap();
        OutboundEmailBody::builder(to)
            .subject(Sentence(1..2).fake::<String>())
            .html_body("<p>HTML Content</p>")
            .text_body(Sentence(1..10).fake::<String>())
            .build()
    }

    /// Get a test instance of `EmailClient`.
    fn email_client(base_url: &str) -> Client {
        let base_url = Url::parse(base_url).expect("Failed to parse base uri");
        let auth_token = 13.fake::<String>();
        let auth_token = SecretString::from(auth_token);

        Client::builder()
            .base_url(base_url)
            .sender(Email::parse(SafeEmail().fake::<String>().as_str()).unwrap())
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
            .send(&build_outbound_email_body())
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
            .send(&build_outbound_email_body())
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
            .send(&build_outbound_email_body())
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
            .send(&build_outbound_email_body())
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
