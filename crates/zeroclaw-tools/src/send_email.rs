use anyhow::Context;
use async_trait::async_trait;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::Deserialize;
use serde_json::{Value, json};
use zeroclaw_api::tool::{Tool, ToolResult};

pub use zeroclaw_config::scattered_types::EmailConfig;

// Import ZeroClaw's core traits (adjust paths based on your exact crate version)
// use zeroclaw::traits::{Tool, ToolResult};

#[derive(Deserialize)]
struct SmtpConfig {
    pub server: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_address: String,
}

pub struct SendEmailTool {
    pub config: Option<EmailConfig>,
}

impl SendEmailTool {
    pub fn new(config: Option<EmailConfig>) -> Self {
        Self { config }
    }

    /// Helper to read the ZeroClaw configuration file from the default path
    fn get_smtp_config(&self) -> Option<SmtpConfig> {
        let config = self.config.as_ref()?;

        let server = config.smtp_host.clone();
        let port = config.smtp_port;
        let username = config.username.clone();
        let password = config.password.clone();
        let from_address = config.from_address.clone();

        Some(SmtpConfig {
            server,
            port,
            username,
            password,
            from_address,
        })
    }
}

#[async_trait]
impl Tool for SendEmailTool {
    fn name(&self) -> &'static str {
        "send_email"
    }

    fn description(&self) -> &'static str {
        "Sends an email to a specified recipient using the SMTP settings configured in config.toml."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "to": {
                    "type": "string",
                    "description": "The recipient's email address"
                },
                "subject": {
                    "type": "string",
                    "description": "The subject line of the email"
                },
                "body": {
                    "type": "string",
                    "description": "The plain text body content of the email"
                }
            },
            "required": ["to", "subject", "body"]
        })
    }

    // Note: If your current version of ZeroClaw strictly expects `Result<ToolResult>`,
    // wrap the returned JSON payload in your ToolResult enum/struct accordingly.
    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        // Extract arguments provided by the AI
        let to = args["to"]
            .as_str()
            .context("Missing or invalid 'to' field")?;
        let subject = args["subject"]
            .as_str()
            .context("Missing or invalid 'subject' field")?;
        let body = args["body"]
            .as_str()
            .context("Missing or invalid 'body' field")?;

        // Load credentials dynamically at execution time
        let smtp_config = self
            .get_smtp_config()
            .context("Email config is not set in config.toml.")?;

        // Build the email message
        let email = Message::builder()
            .from(
                smtp_config
                    .from_address
                    .parse()
                    .context("Invalid 'from_address' in config")?,
            )
            .to(to
                .parse()
                .context("Invalid 'to' address provided by agent")?)
            .subject(subject)
            .body(String::from(body))
            .context("Failed to build the email message")?;

        // Authenticate and construct the async SMTP transport
        let creds = Credentials::new(smtp_config.username, smtp_config.password);
        let mailer: AsyncSmtpTransport<Tokio1Executor> =
            AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp_config.server)
                .context("Failed to resolve SMTP server")?
                .port(smtp_config.port)
                .credentials(creds)
                .build();

        // Dispatch the email
        mailer
            .send(email)
            .await
            .context("Failed to dispatch email via SMTP")?;

        // Return a successful execution state back to the agent loop
        Ok(ToolResult {
            success: true,
            output: String::from("Email successfully dispatched"),
            error: None,
        })
    }
}
