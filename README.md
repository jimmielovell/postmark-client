# Postmark Email Client

A lightweight Rust client for sending emails via the Postmark API.

**⚠️ IMPORTANT NOTE**: This is NOT the official Postmark client. It implements only basic email sending functionality and does not cover the entire Postmark API. For full Postmark API support, please use the [official Postmark client](https://postmarkapp.com/developer).

## Features

- Send individual and batch emails
- Support for email attachments
- Batch sending with automatic size limits (max 500 emails per batch)
- Configurable tracking for opens and link clicks


## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
postmark_client = { git = "https://github.com/jimmielovell/postmark-client"}
```

### Basic Example

```rust
use postmark_client::{Client, Email, OutboundEmailBody, SecretString, Url};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the client
    let client = Client::builder()
        .base_url(Url::parse("https://api.postmarkapp.com")?)
        .sender(Email::parse("sender@example.com")?)
        .auth_token(SecretString::new("your-postmark-token".to_string()))
        .build()?;

    // Create the email body
    let recipient = Email::parse("recipient@example.com")?;
    let email_body = OutboundEmailBody::builder(recipient)
        .subject("Hello from Rust!")
        .html_body("<h1>Hello!</h1><p>This is HTML content</p>")
        .text_body("Hello! This is plain text content")
        .build()?;
    
    // Send the email
    client.send(&email_body).await?;

    Ok(())
}
```

### Advanced Example

```rust
use serde_json::json;
use postmark_client::TrackLink;

let email_body = OutboundEmailBody::builder(recipient)
    .subject("Meeting Summary")
    .html_body("<h1>Meeting Notes</h1>")
    .text_body("Meeting Notes")
    // Optional fields
    .cc(vec![Email::parse("cc@example.com")?])
    .bcc(vec![Email::parse("bcc@example.com")?])
    .reply_to(Email::parse("reply@example.com")?)
    .tag("meeting-notes")
    .metadata(json!({
        "meeting_id": "12345",
        "department": "engineering"
    }))
    .track_opens(true)
    .track_links(TrackLink::HtmlAndText)
    .build()?;
```

### HTML-only or Text-only Emails

You can send emails with just HTML or just text content:

```rust
// HTML-only email
let html_email = OutboundEmailBody::builder(recipient)
    .subject("HTML Newsletter")
    .html_body("<h1>Rich Content</h1>")
    .build()?;

// Text-only email
let text_email = OutboundEmailBody::builder(recipient)
    .subject("Quick Update")
    .text_body("Brief text message")
    .build()?;
```

### Batch Sending

```rust
let email_bodies = vec![
    OutboundEmailBody::builder(recipient1)
        .subject("Update 1")
        .text_body("Message 1")
        .build()?,
    OutboundEmailBody::builder(recipient2)
        .subject("Update 2")
        .text_body("Message 2")
        .build()?,
];

// Send up to 500 emails in one batch
let responses = client.send_batch(&email_bodies).await?;
```

## Limitations

This client:
- Only implements email sending functionality
- Does not support templates
- Does not implement webhook handling
- Does not support message streams
- Does not include statistics or analytics endpoints

For these features, please use other Postmark clients.

## License

Licensed under MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
