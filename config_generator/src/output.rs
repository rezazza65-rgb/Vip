use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::generator::ConfigGenerator;
use crate::tester::{TestResult, TestStatistics};

#[derive(Debug, Serialize, Deserialize)]
pub struct OutputBundle {
    pub subscription_link: String,
    pub configs: Vec<ConfigOutput>,
    pub statistics: TestStatistics,
    pub generated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigOutput {
    pub link: String,
    pub protocol: String,
    pub transmission: String,
    pub address: String,
    pub port: u16,
    pub is_tested: bool,
    pub is_working: bool,
    pub response_time_ms: Option<u64>,
}

pub struct OutputGenerator {
    generator: ConfigGenerator,
}

impl OutputGenerator {
    pub fn new() -> Self {
        Self {
            generator: ConfigGenerator::new(),
        }
    }

    /// Create empty output bundle when no proxies are available
    pub fn create_empty_output(&self) -> OutputBundle {
        let timestamp = chrono::Utc::now().to_rfc3339();
        
        // Create empty subscription (base64 of empty string)
        use base64::{Engine as _, engine::general_purpose};
        let subscription_link = general_purpose::STANDARD.encode("");

        OutputBundle {
            subscription_link,
            configs: Vec::new(),
            statistics: TestStatistics {
                total_configs: 0,
                working_configs: 0,
                failed_configs: 0,
                success_rate: 0.0,
                average_response_time_ms: 0.0,
                fastest_response_time_ms: None,
                slowest_response_time_ms: None,
            },
            generated_at: timestamp,
        }
    }

    pub fn generate_output(&self, test_results: Vec<TestResult>) -> OutputBundle {
        let working_configs: Vec<&TestResult> = test_results
            .iter()
            .filter(|r| r.is_working)
            .collect();

        let mut config_outputs = Vec::new();
        let mut subscription_links = Vec::new();

        for result in &working_configs {
            let link = self.generator.to_subscription_link(&result.config);
            
            subscription_links.push(link.clone());

            let config_output = ConfigOutput {
                link: link.clone(),
                protocol: result.config.protocol.to_string().to_string(),
                transmission: result.config.transmission.to_string().to_string(),
                address: result.config.address.clone(),
                port: result.config.port,
                is_tested: true,
                is_working: result.is_working,
                response_time_ms: result.response_time_ms,
            };

            config_outputs.push(config_output);
        }

        // Create subscription link (base64 encoded list of all configs)
        let subscription_content = subscription_links.join("\n");
        use base64::{Engine as _, engine::general_purpose};
        let subscription_link = general_purpose::STANDARD.encode(&subscription_content);

        let statistics = TestStatistics::from_results(&test_results);
        let timestamp = chrono::Utc::now().to_rfc3339();

        OutputBundle {
            subscription_link,
            configs: config_outputs,
            statistics,
            generated_at: timestamp,
        }
    }

    pub fn save_to_files(&self, output: &OutputBundle, output_dir: &str) -> std::io::Result<()> {
        // Create output directory if it doesn't exist
        fs::create_dir_all(output_dir)?;
        fs::create_dir_all(format!("{}/qr_codes", output_dir))?;

        // Save subscription link
        let subscription_path = Path::new(output_dir).join("subscription.txt");
        fs::write(&subscription_path, &output.subscription_link)?;

        // Save full JSON output
        let json_path = Path::new(output_dir).join("configs.json");
        let json_content = serde_json::to_string_pretty(output)?;
        fs::write(&json_path, json_content)?;

        // Save individual config links in a readable format
        let links_path = Path::new(output_dir).join("config_links.txt");
        if output.configs.is_empty() {
            fs::write(&links_path, "# No working configurations available yet\n# Run workflow again when proxies are detected")?;
        } else {
            let links_content: Vec<String> = output
                .configs
                .iter()
                .map(|c| format!("{}", c.link))
                .collect();
            fs::write(&links_path, links_content.join("\n\n"))?;
        }

        // Save statistics report
        let stats_path = Path::new(output_dir).join("statistics.txt");
        let stats_content = if output.configs.is_empty() {
            format!(
                "Configuration Test Statistics\n\
                 ==============================\n\n\
                 Generated at: {}\n\n\
                 Total Configs Tested: 0\n\
                 Working Configs: 0\n\
                 Failed Configs: 0\n\
                 Success Rate: 0.00%\n\n\
                 Performance Metrics:\n\
                 - Average Response Time: 0.00 ms\n\
                 - Fastest Response Time: N/A\n\
                 - Slowest Response Time: N/A\n\n\
                 Note: No live proxies were detected in the last scan.\n\
                 The system is ready to process proxies once they are discovered.\n",
                output.generated_at,
            )
        } else {
            format!(
                "Configuration Test Statistics\n\
                 ==============================\n\n\
                 Generated at: {}\n\n\
                 Total Configs Tested: {}\n\
                 Working Configs: {}\n\
                 Failed Configs: {}\n\
                 Success Rate: {:.2}%\n\n\
                 Performance Metrics:\n\
                 - Average Response Time: {:.2} ms\n\
                 - Fastest Response Time: {} ms\n\
                 - Slowest Response Time: {} ms\n",
                output.generated_at,
                output.statistics.total_configs,
                output.statistics.working_configs,
                output.statistics.failed_configs,
                output.statistics.success_rate,
                output.statistics.average_response_time_ms,
                output.statistics.fastest_response_time_ms.map(|t| t.to_string()).unwrap_or("N/A".to_string()),
                output.statistics.slowest_response_time_ms.map(|t| t.to_string()).unwrap_or("N/A".to_string()),
            )
        };
        fs::write(&stats_path, stats_content)?;

        // Save configs by protocol
        self.save_configs_by_protocol(output, output_dir)?;

        // Save markdown report
        self.save_markdown_report(output, output_dir)?;

        Ok(())
    }

    fn save_configs_by_protocol(&self, output: &OutputBundle, output_dir: &str) -> std::io::Result<()> {
        let protocols = vec!["VLESS", "VMess", "Trojan", "Shadowsocks"];

        for protocol in protocols {
            let protocol_configs: Vec<&ConfigOutput> = output
                .configs
                .iter()
                .filter(|c| c.protocol == protocol)
                .collect();

            let filename = format!("{}_configs.txt", protocol.to_lowercase());
            let file_path = Path::new(output_dir).join(filename);
            
            if !protocol_configs.is_empty() {
                let content: Vec<String> = protocol_configs
                    .iter()
                    .map(|c| c.link.clone())
                    .collect();
                
                fs::write(&file_path, content.join("\n\n"))?;
            } else {
                // Create empty file with placeholder message
                fs::write(&file_path, format!("# No {} configurations available", protocol))?;
            }
        }

        Ok(())
    }

    fn save_markdown_report(&self, output: &OutputBundle, output_dir: &str) -> std::io::Result<()> {
        let report_path = Path::new(output_dir).join("README.md");
        
        let mut markdown = String::new();
        markdown.push_str("# Proxy Configuration Report\n\n");
        markdown.push_str(&format!("**Generated:** {}\n\n", output.generated_at));
        
        markdown.push_str("## 📊 Statistics\n\n");
        markdown.push_str(&format!("- **Total Configs:** {}\n", output.statistics.total_configs));
        markdown.push_str(&format!("- **Working:** {} {}\n", 
            output.statistics.working_configs,
            if output.statistics.working_configs > 0 { "✅" } else { "⚠️" }
        ));
        markdown.push_str(&format!("- **Failed:** {} {}\n", 
            output.statistics.failed_configs,
            if output.statistics.failed_configs > 0 { "❌" } else { "✓" }
        ));
        markdown.push_str(&format!("- **Success Rate:** {:.2}%\n\n", output.statistics.success_rate));

        if output.configs.is_empty() {
            markdown.push_str("## ⚠️ Status\n\n");
            markdown.push_str("No working proxy configurations are currently available.\n\n");
            markdown.push_str("**Possible Reasons:**\n");
            markdown.push_str("- No live proxies detected by scanner\n");
            markdown.push_str("- All tested configurations failed validation\n");
            markdown.push_str("- Scanner output file not found or empty\n\n");
            markdown.push_str("The system will automatically retry when new proxies are discovered.\n\n");
        } else {
            markdown.push_str("## ⚡ Performance\n\n");
            markdown.push_str(&format!("- **Average Response:** {:.2} ms\n", output.statistics.average_response_time_ms));
            if let Some(fastest) = output.statistics.fastest_response_time_ms {
                markdown.push_str(&format!("- **Fastest:** {} ms\n", fastest));
            }
            if let Some(slowest) = output.statistics.slowest_response_time_ms {
                markdown.push_str(&format!("- **Slowest:** {} ms\n\n", slowest));
            }
        }

        markdown.push_str("## 🔗 Quick Access\n\n");
        markdown.push_str("### Subscription Link\n");
        if !output.subscription_link.is_empty() {
            markdown.push_str("```\n");
            markdown.push_str(&output.subscription_link);
            markdown.push_str("\n```\n\n");
        } else {
            markdown.push_str("*No subscription link available (no working configs)*\n\n");
        }

        markdown.push_str("### Files Available\n\n");
        markdown.push_str("- `subscription.txt` - Base64 encoded subscription link\n");
        markdown.push_str("- `configs.json` - Complete configuration data in JSON format\n");
        markdown.push_str("- `config_links.txt` - All working config links\n");
        markdown.push_str("- `vless_configs.txt` - VLESS protocol configs\n");
        markdown.push_str("- `vmess_configs.txt` - VMess protocol configs\n");
        markdown.push_str("- `trojan_configs.txt` - Trojan protocol configs\n");
        markdown.push_str("- `statistics.txt` - Detailed statistics report\n");
        markdown.push_str("- `qr_codes/` - QR codes for mobile device setup\n\n");

        if !output.configs.is_empty() {
            markdown.push_str("## 📋 Working Configurations\n\n");
            
            // Group by protocol
            let protocols = vec!["VLESS", "VMess", "Trojan"];
            for protocol in protocols {
                let protocol_configs: Vec<&ConfigOutput> = output
                    .configs
                    .iter()
                    .filter(|c| c.protocol == protocol)
                    .collect();

                if !protocol_configs.is_empty() {
                    markdown.push_str(&format!("### {} Configs ({})\n\n", protocol, protocol_configs.len()));
                    
                    for (idx, config) in protocol_configs.iter().enumerate() {
                        markdown.push_str(&format!(
                            "{}. **{}** - {} | {}:{} | ⚡ {}ms\n",
                            idx + 1,
                            config.transmission,
                            config.protocol,
                            config.address,
                            config.port,
                            config.response_time_ms.unwrap_or(0)
                        ));
                    }
                    markdown.push_str("\n");
                }
            }
        }

        markdown.push_str("---\n\n");
        markdown.push_str("*Generated by Advanced Proxy Config Generator*\n");

        fs::write(&report_path, markdown)?;

        Ok(())
    }
}

pub struct SubscriptionManager {
    base_url: Option<String>,
}

impl SubscriptionManager {
    pub fn new(base_url: Option<String>) -> Self {
        Self { base_url }
    }

    pub fn create_subscription_url(&self, subscription_b64: &str) -> Option<String> {
        if let Some(base) = &self.base_url {
            Some(format!("{}/subscription?data={}", base, subscription_b64))
        } else {
            None
        }
    }

    pub fn create_qr_code(&self, link: &str) -> Result<String, Box<dyn std::error::Error>> {
        use qrcode::QrCode;
        use qrcode::render::unicode;

        let code = QrCode::new(link)?;
        let qr_string = code
            .render::<unicode::Dense1x2>()
            .dark_color(unicode::Dense1x2::Light)
            .light_color(unicode::Dense1x2::Dark)
            .build();

        Ok(qr_string)
    }

    pub fn save_qr_codes(&self, configs: &[ConfigOutput], output_dir: &str) -> std::io::Result<()> {
        let qr_dir = Path::new(output_dir).join("qr_codes");
        fs::create_dir_all(&qr_dir)?;

        if configs.is_empty() {
            // Create a placeholder file when no configs are available
            let placeholder_path = qr_dir.join("README.txt");
            fs::write(
                &placeholder_path,
                "QR Code Directory\n\n\
                 No QR codes available yet.\n\
                 QR codes will be generated automatically when working proxy configurations are detected.\n"
            )?;
            return Ok(());
        }

        for (idx, config) in configs.iter().enumerate() {
            if let Ok(qr) = self.create_qr_code(&config.link) {
                let filename = format!("qr_{}_{}.txt", config.protocol.to_lowercase(), idx + 1);
                let file_path = qr_dir.join(filename);
                
                let content = format!(
                    "Config: {} - {}\n\
                     Address: {}:{}\n\n\
                     {}\n\n\
                     Link: {}",
                    config.protocol,
                    config.transmission,
                    config.address,
                    config.port,
                    qr,
                    config.link
                );
                
                fs::write(&file_path, content)?;
            }
        }

        Ok(())
    }
}
