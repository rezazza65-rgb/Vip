use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

use crate::generator::{ConfigGenerator, ProxyConfig};
use crate::tester::{TestResult, TestStatistics};

#[derive(Debug, Serialize, Deserialize)]
pub struct OutputBundle {
    pub subscription_link: String,
    pub working_subscription_link: String,
    pub all_configs: Vec<ConfigOutput>,
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
    pub error_message: Option<String>,
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
        let empty_b64 = general_purpose::STANDARD.encode("");

        OutputBundle {
            subscription_link: empty_b64.clone(),
            working_subscription_link: empty_b64,
            all_configs: Vec::new(),
            configs: Vec::new(),
            statistics: TestStatistics::default(),
            generated_at: timestamp,
        }
    }

    pub fn generate_output_without_testing(&self, configs: &[ProxyConfig]) -> OutputBundle {
        let timestamp = chrono::Utc::now().to_rfc3339();
        let mut all_config_outputs = Vec::new();
        let mut all_links = Vec::new();

        for config in configs {
            let link = self.generator.to_subscription_link(config);
            all_links.push(link.clone());

            let config_output = ConfigOutput {
                link,
                protocol: config.protocol.to_string(),
                transmission: config.transmission.to_string(),
                address: config.address.clone(),
                port: config.port,
                is_tested: false,
                is_working: false,
                response_time_ms: None,
                error_message: None,
            };

            all_config_outputs.push(config_output);
        }

        let subscription_content = all_links.join("\n");
        let subscription_link = general_purpose::STANDARD.encode(&subscription_content);

        OutputBundle {
            subscription_link: subscription_link.clone(),
            working_subscription_link: general_purpose::STANDARD.encode(""),
            all_configs: all_config_outputs,
            configs: Vec::new(),
            statistics: TestStatistics {
                total_configs: configs.len(),
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
        let timestamp = chrono::Utc::now().to_rfc3339();
        
        let mut all_config_outputs = Vec::new();
        let mut working_config_outputs = Vec::new();
        let mut all_links = Vec::new();
        let mut working_links = Vec::new();

        for result in &test_results {
            let link = self.generator.to_subscription_link(&result.config);
            all_links.push(link.clone());

            let config_output = ConfigOutput {
                link: link.clone(),
                protocol: result.config.protocol.to_string(),
                transmission: result.config.transmission.to_string(),
                address: result.config.address.clone(),
                port: result.config.port,
                is_tested: true,
                is_working: result.is_working,
                response_time_ms: result.response_time_ms,
                error_message: result.error_message.clone(),
            };

            all_config_outputs.push(config_output.clone());

            if result.is_working {
                working_links.push(link);
                working_config_outputs.push(config_output);
            }
        }

        let all_subscription_content = all_links.join("\n");
        let subscription_link = general_purpose::STANDARD.encode(&all_subscription_content);

        let working_subscription_content = working_links.join("\n");
        let working_subscription_link = general_purpose::STANDARD.encode(&working_subscription_content);

        let statistics = TestStatistics::from_results(&test_results);

        OutputBundle {
            subscription_link,
            working_subscription_link,
            all_configs: all_config_outputs,
            configs: working_config_outputs,
            statistics,
            generated_at: timestamp,
        }
    }

    pub fn save_all_generated_configs(
        &self,
        output: &OutputBundle,
        output_dir: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(output_dir)?;

        let all_path = Path::new(output_dir).join("all_generated_configs.txt");
        let all_links: Vec<String> = output.all_configs.iter().map(|c| c.link.clone()).collect();
        if !all_links.is_empty() {
            fs::write(&all_path, all_links.join("\n\n"))?;
            println!("      ✓ all_generated_configs.txt ({} configs)", all_links.len());
        } else {
            fs::write(&all_path, "# No configs generated yet")?;
        }

        Ok(())
    }

    pub fn save_to_files(
        &self,
        output: &OutputBundle,
        output_dir: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Ensure directories exist
        fs::create_dir_all(output_dir)?;
        fs::create_dir_all(format!("{}/qr_codes", output_dir))?;

        // subscription.txt
        let subscription_path = Path::new(output_dir).join("subscription.txt");
        fs::write(&subscription_path, &output.subscription_link)?;
        println!("      ✓ subscription.txt");

        // working_subscription.txt
        let working_subscription_path = Path::new(output_dir).join("working_subscription.txt");
        fs::write(&working_subscription_path, &output.working_subscription_link)?;
        println!("      ✓ working_subscription.txt");

        // configs.json
        let json_path = Path::new(output_dir).join("configs.json");
        let json_content = serde_json::to_string_pretty(output)?;
        fs::write(&json_path, json_content)?;
        println!("      ✓ configs.json");

        // all_configs.txt
        let all_configs_path = Path::new(output_dir).join("all_configs.txt");
        if output.all_configs.is_empty() {
            fs::write(&all_configs_path, "# No configurations generated yet\n# Run workflow again when proxies are detected")?;
        } else {
            let all_links: Vec<String> = output.all_configs.iter().map(|c| c.link.clone()).collect();
            fs::write(&all_configs_path, all_links.join("\n\n"))?;
        }
        println!("      ✓ all_configs.txt ({} configs)", output.all_configs.len());

        // config_links.txt (working only)
        let links_path = Path::new(output_dir).join("config_links.txt");
        if output.configs.is_empty() {
            fs::write(&links_path, "# No working configurations available yet\n# All generated configs are in all_configs.txt")?;
        } else {
            let links_content: Vec<String> = output.configs.iter().map(|c| c.link.clone()).collect();
            fs::write(&links_path, links_content.join("\n\n"))?;
        }
        println!("      ✓ config_links.txt ({} working)", output.configs.len());

        // statistics.txt
        let stats_path = Path::new(output_dir).join("statistics.txt");
        let stats_content = self.format_statistics(output);
        fs::write(&stats_path, stats_content)?;
        println!("      ✓ statistics.txt");

        // Protocol-specific files
        self.save_configs_by_protocol(output, output_dir)?;

        // README.md
        self.save_markdown_report(output, output_dir)?;

        Ok(())
    }

    fn format_statistics(&self, output: &OutputBundle) -> String {
        let fastest = output.statistics.fastest_response_time_ms
            .map(|t| format!("{}", t))
            .unwrap_or_else(|| "N/A".to_string());
        let slowest = output.statistics.slowest_response_time_ms
            .map(|t| format!("{}", t))
            .unwrap_or_else(|| "N/A".to_string());

        format!(
            "Configuration Test Statistics\n\
             ==============================\n\n\
             Generated at: {}\n\n\
             Total Configs Generated: {}\n\
             Total Configs Tested: {}\n\
             Working Configs: {}\n\
             Failed Configs: {}\n\
             Success Rate: {:.2}%\n\n\
             Performance Metrics:\n\
             - Average Response Time: {:.2} ms\n\
             - Fastest Response Time: {} ms\n\
             - Slowest Response Time: {} ms\n\n\
             Files Generated:\n\
             - all_configs.txt: All {} generated configurations\n\
             - config_links.txt: {} working configurations\n\
             - subscription.txt: Base64 of all configs\n\
             - working_subscription.txt: Base64 of working configs only\n",
            output.generated_at,
            output.all_configs.len(),
            output.statistics.total_configs,
            output.statistics.working_configs,
            output.statistics.failed_configs,
            output.statistics.success_rate,
            output.statistics.average_response_time_ms,
            fastest,
            slowest,
            output.all_configs.len(),
            output.configs.len(),
        )
    }

    fn save_configs_by_protocol(&self, output: &OutputBundle, output_dir: &str) -> io::Result<()> {
        let protocols = vec![
            ("VLESS", "vless"),
            ("VMess", "vmess"),
            ("Trojan", "trojan"),
            ("SS", "shadowsocks"),
        ];

        for (protocol_name, file_prefix) in protocols {
            let protocol_configs: Vec<&ConfigOutput> = output
                .all_configs
                .iter()
                .filter(|c| c.protocol == protocol_name)
                .collect();

            let filename = format!("{}_configs.txt", file_prefix);
            let file_path = Path::new(output_dir).join(&filename);

            if !protocol_configs.is_empty() {
                let content: Vec<String> = protocol_configs.iter().map(|c| c.link.clone()).collect();
                fs::write(&file_path, content.join("\n\n"))?;
                let working_count = protocol_configs.iter().filter(|c| c.is_working).count();
                println!("      ✓ {} ({} configs, {} working)", filename, protocol_configs.len(), working_count);
            } else {
                fs::write(&file_path, format!("# No {} configurations generated", protocol_name))?;
                println!("      ✓ {} (0 configs)", filename);
            }
        }

        Ok(())
    }

    fn save_markdown_report(&self, output: &OutputBundle, output_dir: &str) -> io::Result<()> {
        let report_path = Path::new(output_dir).join("README.md");

        let mut md = String::new();
        md.push_str("# Proxy Configuration Report\n\n");
        md.push_str(&format!("**Generated:** {}\n\n", output.generated_at));

        md.push_str("## 📊 Statistics\n\n");
        md.push_str(&format!("- **Total Configs Generated:** {}\n", output.all_configs.len()));
        md.push_str(&format!("- **Tested:** {}\n", output.statistics.total_configs));
        md.push_str(&format!("- **Working:** {} {}\n", 
            output.statistics.working_configs,
            if output.statistics.working_configs > 0 { "✅" } else { "⚠️" }
        ));
        md.push_str(&format!("- **Failed:** {}\n", output.statistics.failed_configs));
        md.push_str(&format!("- **Success Rate:** {:.2}%\n\n", output.statistics.success_rate));

        if output.statistics.working_configs > 0 {
            md.push_str("## ⚡ Performance\n\n");
            md.push_str(&format!("- **Average Response:** {:.2} ms\n", output.statistics.average_response_time_ms));
            if let Some(fastest) = output.statistics.fastest_response_time_ms {
                md.push_str(&format!("- **Fastest:** {} ms\n", fastest));
            }
            if let Some(slowest) = output.statistics.slowest_response_time_ms {
                md.push_str(&format!("- **Slowest:** {} ms\n", slowest));
            }
            md.push_str("\n");
        }

        md.push_str("## 📁 Files\n\n");
        md.push_str("| File | Description |\n");
        md.push_str("|------|-------------|\n");
        md.push_str(&format!("| `all_configs.txt` | All {} generated configs |\n", output.all_configs.len()));
        md.push_str(&format!("| `config_links.txt` | {} working configs |\n", output.configs.len()));
        md.push_str("| `subscription.txt` | Base64 of all configs |\n");
        md.push_str("| `working_subscription.txt` | Base64 of working configs |\n");
        md.push_str("| `configs.json` | Complete JSON data |\n");
        md.push_str("| `qr_codes/` | QR codes for mobile |\n\n");

        // Protocol breakdown
        md.push_str("## 📋 By Protocol\n\n");
        for protocol in &["VLESS", "VMess", "Trojan", "SS"] {
            let all = output.all_configs.iter().filter(|c| c.protocol == *protocol).count();
            let working = output.configs.iter().filter(|c| c.protocol == *protocol).count();
            if all > 0 {
                md.push_str(&format!("- **{}**: {} total, {} working\n", protocol, all, working));
            }
        }

        md.push_str("\n---\n*Auto-generated by Proxy Config Generator*\n");

        fs::write(&report_path, md)?;
        println!("      ✓ README.md");

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

        let link_to_encode = if link.len() > 2900 { &link[..2900] } else { link };

        let code = QrCode::new(link_to_encode.as_bytes())
            .map_err(|e| format!("QR: {}", e))?;

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
            fs::write(&placeholder_path, "QR Code Directory\n=================\n\nNo QR codes available yet.\n")?;
            
            let index_path = qr_dir.join("index.txt");
            fs::write(&index_path, "QR Code Index\n=============\n\nNo QR codes generated.\n")?;
            
            println!("      ✓ qr_codes/ (placeholder files created)");
            return Ok(());
        }

        let mut success_count = 0;
        let mut error_count = 0;

        let max_qr_codes = std::cmp::min(100, configs.len());
        
        for (idx, config) in configs.iter().take(max_qr_codes).enumerate() {
            match self.create_qr_code(&config.link) {
                Ok(qr) => {
                    let status = if config.is_working { "working" } else { "generated" };
                    let filename = format!("qr_{:03}_{}_{}.txt", idx + 1, config.protocol.to_lowercase(), status);
                    let file_path = qr_dir.join(&filename);

                    let response_info = config.response_time_ms
                        .map(|t| format!("{} ms", t))
                        .unwrap_or_else(|| "N/A".to_string());

                    let status_str = if config.is_working { "✅ Working" } else { "⚠️ Generated" };

                    let content = format!(
                        "Config #{} - {} - {}\n{}\n\n\
                         Protocol: {}\n\
                         Transmission: {}\n\
                         Address: {}:{}\n\
                         Status: {}\n\
                         Response Time: {}\n\n\
                         QR Code:\n{}\n\n\
                         Link:\n{}",
                        idx + 1, config.protocol, config.transmission,
                        "=".repeat(40),
                        config.protocol, config.transmission,
                        config.address, config.port,
                        status_str, response_info,
                        qr, config.link
                    );

                    if fs::write(&file_path, content).is_ok() {
                        success_count += 1;
                    } else {
                        error_count += 1;
                    }
                }
                Err(_) => {
                    error_count += 1;
                }
            }
        }

        // Create index file
        let index_path = qr_dir.join("index.txt");
        let timestamp = chrono::Utc::now().to_rfc3339();
        let index_content = format!(
            "QR Code Index\n=============\n\n\
             Generated at: {}\n\n\
             Total QR codes: {}\n\
             Errors: {}\n\n\
             Total configs available: {}\n",
            timestamp, success_count, error_count, configs.len()
        );
        fs::write(&index_path, index_content)?;

        println!("      ✓ qr_codes/ ({} generated, {} errors)", success_count, error_count);
        Ok(())
    }
}
