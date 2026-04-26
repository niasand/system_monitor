use std::time::Duration;

use anyhow::Result;

pub struct FeishuNotifier {
    webhook_url: String,
    agent: ureq::Agent,
}

impl FeishuNotifier {
    pub fn new(webhook_url: String) -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout_read(Duration::from_secs(10))
            .timeout_write(Duration::from_secs(5))
            .build();
        Self { webhook_url, agent }
    }

    pub fn send(&self, message: &serde_json::Value) -> Result<()> {
        self.agent
            .post(&self.webhook_url)
            .send_json(message)
            .map_err(|e| anyhow::anyhow!("feishu webhook failed: {e}"))?;
        Ok(())
    }
}
