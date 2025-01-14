# Postmark Email Client

A lightweight Rust client for sending emails via the Postmark API.

**⚠️ IMPORTANT NOTE**: This is NOT the official Postmark client. It implements only basic email sending functionality and does not cover the entire Postmark API. For full Postmark API support, please use the [official Postmark client](https://postmarkapp.com/developer).

## Features

- Send emails with HTML and plain text content
- Support for email attachments
- Configurable timeouts
- Type-safe error handling

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
postmark_client = { git = "https://github.com/jimmielovell/postmark-client"}
```

### Basic Example

```rust
use postmark_client::{Client, Email};
use secrecy::SecretString;
use reqwest::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .base_url(Url::parse("https://api.postmarkapp.com")?)
        .sender(Email::parse("sender@example.com")?)
        .auth_token(SecretString::new("your-postmark-token".to_string()))
        .build()?;

    let recipient = Email::parse("recipient@example.com")?;
    
    client.send(
        &recipient,
        "Hello from Rust!",
        "<h1>Hello!</h1><p>This is HTML content</p>",
        "Hello! This is plain text content",
        None,
        None,
    ).await?;

    Ok(())
}
```

### With Attachments

```rust
use postmark_client::Attachment;

let attachment = Attachment::from_file(
    "document.txt",
    "path/to/document.txt"
)?;

client.send(
    &recipient,
    "Email with attachment",
    "HTML content",
    "Text content",
    None,
    Some(vec![attachment]),
).await?;
```

## Configuration Options

The client can be configured with several options:

- `timeout`: Maximum duration to wait for API response
- `max_retries`: Number of retry attempts for failed requests
- Custom HTTP client configuration
- Request retry behavior

## Error Handling

The client provides detailed error types for different failure scenarios:

- Network errors
- Authentication failures
- Server errors
- Configuration issues
- Invalid attachments

## Limitations

This client:
- Only implements basic email sending functionality
- Does not support templates
- Does not support batch sending
- Does not implement webhook handling
- Does not support message streams
- Does not include statistics or analytics endpoints

For these features, please use the other Postmark clients.

## License

Licensed under MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
