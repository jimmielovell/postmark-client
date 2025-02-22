use crate::error::ParseError;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

static RE_START_CHAR: Lazy<Regex> = Lazy::new(|| Regex::new(r"[a-zA-Z1-9]").unwrap());
static RE_DOT_TLD: Lazy<Regex> =
    Lazy::new(|| Regex::new(r".+?@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap());
static RE_VALID: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap());

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Email(String);

impl Email {
    pub fn parse(email: &str) -> Result<Email, ParseError> {
        // Remove any trailing whitespaces
        let email = email.trim();
        if email.is_empty() {
            return Err(ParseError("an email cannot be empty".to_string()));
        }

        let re_start_char = Lazy::force(&RE_START_CHAR);
        if !re_start_char.is_match(&email[0..1]) {
            return Err(ParseError(
                "an email can only start with a letter or a number".to_string(),
            ));
        }

        let re_dot_tld = Lazy::force(&RE_DOT_TLD);
        if !re_dot_tld.is_match(email) {
            return Err(ParseError(
                "the email does not contain a valid [domain] part".to_string(),
            ));
        }

        let re_valid = Lazy::force(&RE_VALID);
        if !re_valid.is_match(email) {
            return Err(ParseError(format!("{email} is not a valid email")));
        }

        Ok(Email(email.to_string().to_lowercase()))
    }

    /// Should just be used internally when an email value is already known
    /// to be from a valid source. e.g. from a watfoe database
    pub fn parse_unsafe(email: String) -> Email {
        Email(email)
    }

    pub fn hash(&self) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(self.0.as_bytes());
        hasher.finalize().to_string()
    }
}

impl AsRef<str> for Email {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl PartialEq<Email> for Email {
    fn eq(&self, other: &Email) -> bool {
        self.0 == other.0
    }
}

#[cfg(test)]
mod tests {
    use crate::email::Email;
    use claim::assert_err;
    use fake::faker::internet::en::SafeEmail;
    use fake::Fake;
    use quickcheck::{Arbitrary, Gen};

    #[derive(Debug, Clone)]
    struct ValidEmailFixture(pub String);

    impl Arbitrary for ValidEmailFixture {
        fn arbitrary(_g: &mut Gen) -> ValidEmailFixture {
            let email = SafeEmail().fake();
            Self(email)
        }
    }

    #[test]
    fn empty_string_is_rejected() {
        let email = "";
        assert_err!(Email::parse(email));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "jimmieomlovelldomain.com";
        assert_err!(Email::parse(email));
    }

    #[test]
    fn email_missing_domain_is_rejected() {
        let test_cases = vec!["jimmielovell@domaincom", "jimmielovell"];

        for email in test_cases {
            assert_err!(Email::parse(email));
        }
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@domains.com";
        assert_err!(Email::parse(email));
    }

    #[test]
    fn email_not_starting_with_letter_is_rejected() {
        let test_cases = vec![
            "!jimmielovell@domains.com",
            "@jimmielovell@domains.com",
            "#jimmielovell@domains.com",
            "$jimmielovell@domains.com",
            "%jimmielovell@domains.com",
            "^jimmielovell@domains.com",
            "&jimmielovell@domains.com",
            "*jimmielovell@domains.com",
            "(jimmielovell@domains.com",
            ")jimmielovell@domains.com",
            "_jimmielovell@domains.com",
            "+jimmielovell@domains.com",
            "~jimmielovell@domains.com",
            "`jimmielovell@domains.com",
        ];

        for email in test_cases {
            assert_err!(Email::parse(email));
        }
    }

    #[quickcheck_macros::quickcheck]
    fn a_valid_email_is_parsed_successfully(valid_email: ValidEmailFixture) -> bool {
        Email::parse(valid_email.0.as_str()).is_ok()
    }
}
