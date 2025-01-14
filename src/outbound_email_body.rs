use crate::attachment::Attachment;
use crate::Email;
use serde_json::Value;

#[derive(Debug, Clone, Copy)]
pub enum TrackLink {
    None,
    HtmlAndText,
    HtmlOnly,
    TextOnly,
}

impl TrackLink {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            TrackLink::None => "None",
            TrackLink::HtmlAndText => "HtmlAndText",
            TrackLink::HtmlOnly => "HtmlOnly",
            TrackLink::TextOnly => "TextOnly",
        }
    }
}

#[derive(Debug)]
pub struct OutboundEmailBody {
    pub(crate) to: Email,
    pub(crate) subject: Option<String>,
    pub(crate) cc: Option<Vec<Email>>,
    pub(crate) bcc: Option<Vec<Email>>,
    pub(crate) tag: Option<String>,
    pub(crate) html_body: Option<String>,
    pub(crate) text_body: Option<String>,
    pub(crate) reply_to: Option<Email>,
    pub(crate) metadata: Option<Value>,
    pub(crate) track_opens: bool,
    pub(crate) track_links: TrackLink,
    pub(crate) attachments: Option<Vec<Attachment>>,
}

impl OutboundEmailBody {
    pub fn builder(to: Email) -> OutboundEmailBodyBuilder {
        OutboundEmailBodyBuilder::new(to)
    }
}

// The builder for OutboundEmailBody
pub struct OutboundEmailBodyBuilder {
    to: Email,
    subject: Option<String>,
    cc: Option<Vec<Email>>,
    bcc: Option<Vec<Email>>,
    tag: Option<String>,
    html_body: Option<String>,
    text_body: Option<String>,
    reply_to: Option<Email>,
    metadata: Option<Value>,
    track_opens: bool,
    track_links: TrackLink,
    attachments: Option<Vec<Attachment>>,
}

impl OutboundEmailBodyBuilder {
    pub fn new(to: Email) -> Self {
        Self {
            to,
            subject: None,
            html_body: None,
            text_body: None,
            cc: None,
            bcc: None,
            tag: None,
            reply_to: None,
            metadata: None,
            track_opens: true,
            track_links: TrackLink::HtmlAndText,
            attachments: None,
        }
    }

    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    pub fn html_body(mut self, html_body: impl Into<String>) -> Self {
        self.html_body = Some(html_body.into());
        self
    }

    pub fn text_body(mut self, text_body: impl Into<String>) -> Self {
        self.text_body = Some(text_body.into());
        self
    }

    pub fn cc(mut self, cc: Vec<Email>) -> Self {
        self.cc = Some(cc);
        self
    }

    pub fn bcc(mut self, bcc: Vec<Email>) -> Self {
        self.bcc = Some(bcc);
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    pub fn reply_to(mut self, reply_to: Email) -> Self {
        self.reply_to = Some(reply_to);
        self
    }

    pub fn metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn track_opens(mut self, track_opens: bool) -> Self {
        self.track_opens = track_opens;
        self
    }

    pub fn track_links(mut self, track_links: TrackLink) -> Self {
        self.track_links = track_links;
        self
    }

    pub fn attachments(mut self, attachments: Vec<Attachment>) -> Self {
        self.attachments = Some(attachments);
        self
    }

    pub fn build(self) -> OutboundEmailBody {
        OutboundEmailBody {
            to: self.to,
            subject: self.subject,
            cc: self.cc,
            bcc: self.bcc,
            tag: self.tag,
            html_body: self.html_body,
            text_body: self.text_body,
            reply_to: self.reply_to,
            metadata: self.metadata,
            track_opens: self.track_opens,
            track_links: self.track_links,
            attachments: self.attachments,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_email_request_builder() {
        let to = Email::parse("to@example.com").unwrap();
        let cc_email = Email::parse("cc@example.com").unwrap();
        let reply_to = Email::parse("reply@example.com").unwrap();

        let request = OutboundEmailBody::builder(to)
            .subject("Test Subject")
            .html_body("<p>HTML Content</p>")
            .text_body("Text Content")
            .cc(vec![cc_email])
            .reply_to(reply_to)
            .tag("test-tag".to_string())
            .track_opens(false)
            .track_links(TrackLink::HtmlOnly)
            .metadata(json!({ "key": "value" }))
            .build();

        assert_eq!(request.subject, Some("Test Subject".to_string()));
        assert_eq!(request.to.as_ref(), "to@example.com");
        assert_eq!(request.cc.as_ref().unwrap()[0].as_ref(), "cc@example.com");
        assert_eq!(request.reply_to.unwrap().as_ref(), "reply@example.com");
        assert_eq!(request.tag.unwrap(), "test-tag");
        assert!(!request.track_opens);
        assert!(matches!(request.track_links, TrackLink::HtmlOnly));
    }
}
