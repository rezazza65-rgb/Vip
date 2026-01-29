use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for OutputGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputGenerator {
    pub fn new() -> Self {
        Self {
            generator: ConfigGenerator::new(),
        }
    }

    pub fn create_empty_output(&self) -> OutputBundle {
        let timestamp = chrono::Utc::now().to_rfc3339();
        let subscription_link = general_purpose::STANDARD.encode("");

        OutputBundle {
            subscription_link,
            configs: Vec::new(),
            statistics: TestStatistics::default(),
            generated_at: timestamp,
        }
    }

    pub fn generate_output(&self, test_results: Vec<TestResult>) -> OutputBundle {
        let working_configs: Vec<&TestResult> =
            test_results.iter().filter(|r| r.is_working).collect();

        let mut config_outputs = Vec::new();
        let mut subscription_links = Vec::new();

        for result in &working_configs {
            let link = self.generator.to_subscription_link(&result.config);

            subscription_links.push(link.clone());

            let config_output = ConfigOutput {
                link,
                protocol: result.config.protocol.to_string(),
                transmission: result.config.transmission.to_string(),
                address: result.config.address.clone(),
                port: result.config.port,
                is_tested: true,
                is_working: result.is_working,
                response_time_ms: result.response_time_ms,
            };

            config_outputs.push(config_output);
        }

        let subscription_content = subscription_links.join("\n");
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

    pub fn save_to_files(
        &self,
        output: &OutputBundle,
        output_dir: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(output_dir)?;
        fs::create_dir_all(format!("{}/qr_codes", output_dir))?;

        // subscription.txt
        let subscription_path = Path::new(output_dir).join("subscription.txt");
        fs::write(&subscription_path, &output.subscription_link)?;
        println!("   ✓ subscription.txt");

        // configs.json
        let json_path = Path::new(output_dir).join("configs.json");
        let json_content = serde_json::to_string_pretty(output)?;
        fs::write(&json_path, json_content)?;
        println!("   ✓ configs.json");

        // config_links.txt
        let links_path = Path::new(output_dir).join("config_links.txt");
        if output.configs.is_empty() {
            fs::write(
                &links_path,
                "# No working configurations available yet\n# Run workflow again when proxies are detected",
            )?;
        } else {
            let links_content: Vec<String> =
                output.configs.iter().map(|c| c.link.clone()).collect();
            fs::write(&links_path, links_content.join("\n\n"))?;
        }
        println!("   ✓ config_links.txt");

        // statistics.txt
        let stats_path = Path::new(output_dir).join("statistics.txt");
        let stats_content = self.format_statistics(output);
        fs::write(&stats_path, stats_content)?;
        println!("   ✓ statistics.txt");

        // Protocol-specific files
        self.save_configs_by_protocol(output, output_dir)?;

        // README.md
        self.save_markdown_report(output, output_dir)?;

        Ok(())
    }

    fn format_statistics(&self, output: &OutputBundle) -> String {
        if output.configs.is_empty() {
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
            let fastest = output
                .statistics
                .fastest_response_time_ms
                .map(|t| format!("{}", t))
                .unwrap_or_else(|| "N/A".to_string());
            let slowest = output
                .statistics
                .slowest_response_time_ms
                .map(|t| format!("{}", t))
                .unwrap_or_else(|| "N/A".to_string());

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
                fastest,
                slowest,
            )
        }
    }

    fn save_configs_by_protocol(
        &self,
        output: &OutputBundle,
        output_dir: &str,
    ) -> io::Result<()> {
        let protocols = vec![
            ("VLESS", "vless"),
            ("VMess", "vmess"),
            ("Trojan", "trojan"),
            ("SS", "shadowsocks"),
        ];

        for (protocol_name, file_prefix) in protocols {
            let protocol_configs: Vec<&ConfigOutput> = output
                .configs
                .iter()
                .filter(|c| c.protocol == protocol_name)
                .collect();

            let filename = format!("{}_configs.txt", file_prefix);
            let file_path = Path::new(output_dir).join(&filename);

            if !protocol_configs.is_empty() {
                let content: Vec<String> =
                    protocol_configs.iter().map(|c| c.link.clone()).collect();
                fs::write(&file_path, content.join("\n\n"))?;
            } else {
                fs::write(
                    &file_path,
                    format!("# No {} configurations available", protocol_name),
                )?;
            }
            println!("   ✓ {}", filename);
        }

        Ok(())
    }

    fn save_markdown_report(
        &self,
        output: &OutputBundle,
        output_dir: &str,
    ) -> io::Result<()> {
        let report_path = Path::new(output_dir).join("README.md");

        let mut md = String::new();
        md.push_str("# Proxy Configuration Report\n\n");
        md.push_str(&format!("**Generated:** {}\n\n", output.generated_at));

        md.push_str("## 📊 Statistics\n\n");
        md.push_str(&format!(
            "- **Total Configs:** {}\n",
            output.statistics.total_configs
        ));
        md.push_str(&format!(
            "- **Working:** {} {}\n",
            output.statistics.working_configs,
            if output.statistics.working_configs > 0 {
                "✅"
            } else {
                "⚠️"
            }
        ));
        md.push_str(&format!(
            "- **Failed:** {} {}\n",
            output.statistics.failed_configs,
            if output.statistics.failed_configs > 0 {
                "❌"
            } else {
                "✓"
            }
        ));
        md.push_str(&format!(
            "- **Success Rate:** {:.2}%\n\n",
            output.statistics.success_rate
        ));

        if output.configs.is_empty() {
            md.push_str("## ⚠️ Status\n\n");
            md.push_str("No working proxy configurations are currently available.\n\n");
            md.push_str("**Possible Reasons:**\n");
            md.push_str("- No live proxies detected by scanner\n");
            md.push_str("- All tested configurations failed validation\n");
            md.push_str("- Scanner output file not found or empty\n\n");
            md.push_str(
                "The system will automatically retry when new proxies are discovered.\n\n",
            );
        } else {
            md.push_str("## ⚡ Performance\n\n");
            md.push_str(&format!(
                "- **Average Response:** {:.2} ms\n",
                output.statistics.average_response_time_ms
            ));
            if let Some(fastest) = output.statistics.fastest_response_time_ms {
                md.push_str(&format!("- **Fastest:** {} ms\n", fastest));
            }
            if let Some(slowest) = output.statistics.slowest_response_time_ms {
                md.push_str(&format!("- **Slowest:** {} ms\n\n", slowest));
            }
        }

        md.push_str("## 🔗 Quick Access\n\n");
        md.push_str("### Subscription Link\n");

        let empty_b64 = general_purpose::STANDARD.encode("");
        if !output.subscription_link.is_empty() && output.subscription_link != empty_b64 {
            md.push_str("```\n");
            md.push_str(&output.subscription_link);
            md.push_str("\n```\n\n");
        } else {
            md.push_str("*No subscription link available (no working configs)*\n\n");
        }

        md.push_str("### Files Available\n\n");
        md.push_str("- `subscription.txt` - Base64 encoded subscription link\n");
        md.push_str("- `configs.json` - Complete configuration data in JSON format\n");
        md.push_str("- `config_links.txt` - All working config links\n");
        md.push_str("- `vless_configs.txt` - VLESS protocol configs\n");
        md.push_str("- `vmess_configs.txt` - VMess protocol configs\n");
        md.push_str("- `trojan_configs.txt` - Trojan protocol configs\n");
        md.push_str("- `shadowsocks_configs.txt` - Shadowsocks protocol configs\n");
        md.push_str("- `statistics.txt` - Detailed statistics report\n");
        md.push_str("- `qr_codes/` - QR codes for mobile device setup\n\n");

        if !output.configs.is_empty() {
            md.push_str("## 📋 Working Configurations\n\n");

            let protocols = vec!["VLESS", "VMess", "Trojan", "SS"];
            for protocol in protocols {
                let protocol_configs: Vec<&ConfigOutput> = output
                    .configs
                    .iter()
                    .filter(|c| c.protocol == protocol)
                    .collect();

                if !protocol_configs.is_empty() {
                    md.push_str(&format!(
                        "### {} Configs ({})\n\n",
                        protocol,
                        protocol_configs.len()
                    ));

                    for (idx, config) in protocol_configs.iter().enumerate() {
                        md.push_str(&format!(
                            "{}. **{}** - {} | {}:{} | ⚡ {}ms\n",
                            idx + 1,
                            config.transmission,
                            config.protocol,
                            config.address,
                            config.port,
                            config.response_time_ms.unwrap_or(0)
                        ));
                    }
                    md.push_str("\n");
                }
            }
        }

        md.push_str("---\n\n");
        md.push_str("*Generated by Advanced Proxy Config Generator*\n");

        fs::write(&report_path, md)?;
        println!("   ✓ README.md");

        Ok(())
    }
}

pub struct SubscriptionManager {
    #[allow(dead_code)]
    base_url: Option<String>,
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new(None)
    }
}

impl SubscriptionManager {
    pub fn new(base_url: Option<String>) -> Self {
        Self { base_url }
    }

    pub fn create_qr_code(&self, link: &str) -> Result<String, String> {
        use qrcode::render::unicode;
        use qrcode::QrCode;

        let link_to_encode = if link.len() > 2900 {
            &link[..2900]
        } else {
            link
        };

        let code = QrCode::new(link_to_encode.as_bytes())
            .map_err(|e| format!("QR code creation failed: {}", e))?;

        let qr_string = code
            .render::<unicode::Dense1x2>()
            .dark_color(unicode::Dense1x2::Light)
            .light_color(unicode::Dense1x2::Dark)
            .build();

        Ok(qr_string)
    }

    pub fn save_qr_codes(
        &self,
        configs: &[ConfigOutput],
        output_dir: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let qr_dir = Path::new(output_dir).join("qr_codes");
        fs::create_dir_all(&qr_dir)?;

        if configs.is_empty() {
            let placeholder_path = qr_dir.join("README.txt");
            fs::write(
                &placeholder_path,
                "QR Code Directory\n\n\
                 No QR codes available yet.\n\
                 QR codes will be generated automatically when working proxy configurations are detected.\n",
            )?;
            println!("   ✓ qr_codes/README.txt (placeholder)");
            return Ok(());
        }

        let mut success_count = 0;
        let mut error_count = 0;

        for (idx, config) in configs.iter().enumerate() {
            match self.create_qr_code(&config.link) {
                Ok(qr) => {
                    let filename =
                        format!("qr_{}_{}.txt", config.protocol.to_lowercase(), idx + 1);
                    let file_path = qr_dir.join(&filename);

                    let content = format!(
                        "Config: {} - {}\n\
                         Address: {}:{}\n\
                         Response Time: {} ms\n\n\
                         {}\n\n\
                         Link:\n{}",
                        config.protocol,
                        config.transmission,
                        config.address,
                        config.port,
                        config.response_time_ms.unwrap_or(0),
                        qr,
                        config.link
                    );

                    match fs::write(&file_path, content) {
                        Ok(_) => success_count += 1,
                        Err(e) => {
                            eprintln!("   ⚠️ Failed to write {}: {}", filename, e);
                            error_count += 1;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("   ⚠️ QR generation failed for config {}: {}", idx + 1, e);
                    error_count += 1;
                }
            }
        }

        println!(
            "   ✓ qr_codes/ ({} generated, {} errors)",
            success_count, error_count
        );
        Ok(())
    }
}
