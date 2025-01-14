use crate::error::ClientError;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Default)]
pub struct AttachmentBuilder {
    name: Option<String>,
    content: Option<Vec<u8>>,
    content_type: Option<String>,
    content_id: Option<String>,
}

impl AttachmentBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn content(mut self, content: Vec<u8>) -> Self {
        self.content = Some(content);
        self
    }

    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }

    pub fn content_id(mut self, content_id: impl Into<String>) -> Self {
        self.content_id = Some(content_id.into());
        self
    }

    pub fn build(self) -> Result<Attachment, ClientError> {
        let name = self
            .name
            .ok_or_else(|| ClientError::Configuration("attachment name is required".to_string()))?;
        let content = self.content.ok_or_else(|| {
            ClientError::Configuration("attachment content is required".to_string())
        })?;
        let content_type = self.content_type.ok_or_else(|| {
            ClientError::Configuration("attachment content type is required".to_string())
        })?;

        Ok(Attachment {
            name,
            content: base64::engine::general_purpose::STANDARD.encode(content),
            content_type,
            content_id: self.content_id,
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Attachment {
    name: String,
    content: String,
    content_type: String,
    #[serde(rename = "ContentID")]
    content_id: Option<String>,
}

impl Attachment {
    pub fn builder() -> AttachmentBuilder {
        AttachmentBuilder::new()
    }

    pub fn from_file(name: &str, filename: &str) -> Result<Self, ClientError> {
        let content = fs::read(filename).map_err(ClientError::Io)?;
        let ext = std::path::Path::new(filename)
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .ok_or_else(|| ClientError::Configuration("Invalid file extension".to_string()))?;

        let content_type = mime_guess::from_ext(ext)
            .first_or_octet_stream()
            .to_string();

        Self::builder()
            .name(name.to_owned())
            .content(content)
            .content_type(content_type)
            .build()
    }
}
