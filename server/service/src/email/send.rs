use lettre::{
    Message, SmtpTransport, Transport,
    address::AddressError,
    message::{Mailbox, MultiPart},
};

// This enum defines the errors that can occur when sending an email.
// It provides a is_permanent method to check if the error is permanent or temporary.
#[derive(Debug)]
pub enum EmailSendError {
    AddressError,
    MessageBuildError,
    SmtpError(lettre::transport::smtp::Error),
}

impl EmailSendError {
    pub fn is_permanent(&self) -> bool {
        match self {
            EmailSendError::AddressError => true,
            EmailSendError::MessageBuildError => true,
            EmailSendError::SmtpError(e) => e.is_permanent(),
        }
    }
}

/// A recipient string is one or more RFC 5322 mailboxes separated by `;`
/// (e.g. `a@example.com; Bee <b@example.com>`). Blank entries (a trailing `;`)
/// are ignored; at least one valid address is required and any invalid one is
/// a (permanent) AddressError. Commas are not separators: they can appear
/// inside a quoted display name.
fn parse_recipients(to: &str) -> Result<Vec<Mailbox>, EmailSendError> {
    let recipients = to
        .split(';')
        .map(str::trim)
        .filter(|address| !address.is_empty())
        .map(|address| {
            address
                .parse::<Mailbox>()
                .map_err(|_e: AddressError| EmailSendError::AddressError)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if recipients.is_empty() {
        return Err(EmailSendError::AddressError);
    }
    Ok(recipients)
}

/**
    send_email takes a mailer (provided as a SmtpTransport), a from address (provided as a Mailbox),
    with a subject (provided as a string) and a body (provided as a string).
    It returns an error format with either a permanent error (which should be logged and not retried)
    or a temporary error (which should be logged and retried).
*/
pub fn send_email(
    mailer: &SmtpTransport,
    from: Mailbox,
    to: String,
    subject: String,
    html_body: String,
    text_body: String,
) -> Result<(), EmailSendError> {
    let recipients = parse_recipients(&to)?;

    let mut builder = Message::builder().from(from);
    for recipient in recipients {
        builder = builder.to(recipient);
    }
    let message = builder
        .subject(subject)
        .multipart(MultiPart::alternative_plain_html(text_body, html_body))
        .map_err(|_e| EmailSendError::MessageBuildError)?;

    mailer.send(&message).map_err(EmailSendError::SmtpError)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_recipients_single_address() {
        let recipients = parse_recipients("a@example.com").unwrap();
        assert_eq!(recipients.len(), 1);
        assert_eq!(recipients[0].email.to_string(), "a@example.com");
    }

    #[test]
    fn parse_recipients_semicolon_separated() {
        let recipients =
            parse_recipients("a@example.com; Bee <b@example.com>;c@example.com;").unwrap();
        let emails: Vec<String> = recipients.iter().map(|r| r.email.to_string()).collect();
        assert_eq!(
            emails,
            vec!["a@example.com", "b@example.com", "c@example.com"]
        );
    }

    #[test]
    fn parse_recipients_rejects_invalid_or_empty() {
        assert!(matches!(
            parse_recipients("a@example.com;not-an-address"),
            Err(EmailSendError::AddressError)
        ));
        assert!(matches!(
            parse_recipients(" ; "),
            Err(EmailSendError::AddressError)
        ));
    }
}
