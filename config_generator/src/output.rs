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
    pub configs: Vec<ConfigOutput>,  // Working configs only (for backward compatibility)
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
        let subscription_link = general_purpose::STANDARD.encode("");

        OutputBundle {
            subscription_link: subscription_link.clone(),
            working_subscription_link: subscription_link,
            all_configs: Vec::new(),
            configs: Vec::new(),
            statistics: TestStatistics::default(),
            generated_at: timestamp,
        }
    }

    /// Generate output without testing - just creates configs
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

    /// Save all generated configs before testing
    pub fn save_all_generated_configs(
        &self,
        output: &OutputBundle,
        output_dir: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(output_dir)?;

        // all_generated_configs.txt - All configs before testing
        let all_path = Path::new(output_dir).join("all_generated_configs.txt");
        let all_links: Vec<String> = output.all_configs.iter().map(|c| c.link.clone()).collect();
        if !all_links.is_empty() {
            fs::write(&all_path, all_links.join("\n\n"))?;
            println!("   ✓ all_generated_configs.txt ({} configs)", all_links.len());
        }

        Ok(())
    }

    pub fn save_to_files(
        &self,
        output: &OutputBundle,
        output_dir: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(output_dir)?;
        fs::create_dir_all(format!("{}/qr_codes", output_dir))?;

        // subscription.txt - Base64 of ALL configs
        let subscription_path = Path::new(output_dir).join("subscription.txt");
        fs::write(&subscription_path, &output.subscription_link)?;
        println!("   ✓ subscription.txt (all configs)");

        // working_subscription.txt - Base64 of only working configs
        let working_subscription_path = Path::new(output_dir).join("working_subscription.txt");
        fs::write(&working_subscription_path, &output.working_subscription_link)?;
        println!("   ✓ working_subscription.txt (working only)");

        // configs.json - Complete data with both all and working configs
        let json_path = Path::new(output_dir).join("configs.json");
        let json_content = serde_json::to_string_pretty(output)?;
        fs::write(&json_path, json_content)?;
        println!("   ✓ configs.json");

        // all_configs.txt - All generated configs
        let all_configs_path = Path::new(output_dir).join("all_configs.txt");
        if output.all_configs.is_empty() {
            fs::write(
                &all_configs_path,
                "# No configurations generated yet\n# Run workflow again when proxies are detected",
            )?;
        } else {
            let all_links: Vec<String> = output.all_configs.iter().map(|c| c.link.clone()).collect();
            fs::write(&all_configs_path, all_links.join("\n\n"))?;
        }
        println!("   ✓ all_configs.txt ({} configs)", output.all_configs.len());

        // config_links.txt - Working configs only (for backward compatibility)
        let links_path = Path::new(output_dir).join("config_links.txt");
        if output.configs.is_empty() {
            fs::write(
                &links_path,
                "# No working configurations available yet\n# All generated configs are in all_configs.txt",
            )?;
        } else {
            let links_content: Vec<String> =
                output.configs.iter().map(|c| c.link.clone()).collect();
            fs::write(&links_path, links_content.join("\n\n"))?;
        }
        println!("   ✓ config_links.txt ({} working)", output.configs.len());

        // statistics.txt
        let stats_path = Path::new(output_dir).join("statistics.txt");
        let stats_content = self.format_statistics(output);
        fs::write(&stats_path, stats_content)?;
        println!("   ✓ statistics.txt");

        // Protocol-specific files (include ALL configs, not just working)
        self.save_configs_by_protocol(output, output_dir)?;

        // README.md
        self.save_markdown_report(output, output_dir)?;

        Ok(())
    }

    fn format_statistics(&self, output: &OutputBundle) -> String {
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
            // Save ALL configs of this protocol (not just working)
            let protocol_configs: Vec<&ConfigOutput> = output
                .all_configs
                .iter()
                .filter(|c| c.protocol == protocol_name)
                .collect();

            let filename = format!("{}_configs.txt", file_prefix);
            let file_path = Path::new(output_dir).join(&filename);

            if !protocol_configs.is_empty() {
                let content: Vec<String> =
                    protocol_configs.iter().map(|c| c.link.clone()).collect();
                fs::write(&file_path, content.join("\n\n"))?;
                
                // Count working configs for logging
                let working_count = protocol_configs.iter().filter(|c| c.is_working).count();
                println!("   ✓ {} ({} configs, {} working)", filename, protocol_configs.len(), working_count);
            } else {
                fs::write(
                    &file_path,
                    format!("# No {} configurations generated", protocol_name),
                )?;
                println!("   ✓ {} (0 configs)", filename);
            }
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
            "- **Total Configs Generated:** {}\n",
            output.all_configs.len()
        ));
        md.push_str(&format!(
            "- **Tested:** {}\n",
            output.statistics.total_configs
        ));
        md.push_str(&format!(
            "- **Working:** {} {}\n",
            output.statistics.working_configs,
            if output.statistics.working_configs > 0 { "✅" } else { "⚠️" }
        ));
        md.push_str(&format!(
            "- **Failed:** {} {}\n",
            output.statistics.failed_configs,
            if output.statistics.failed_configs > 0 { "❌" } else { "✓" }
        ));
        md.push_str(&format!(
            "- **Success Rate:** {:.2}%\n\n",
            output.statistics.success_rate
        ));

        if output.statistics.working_configs > 0 {
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

        md.push_str("## 📁 Files Available\n\n");
        md.push_str("### Subscription Links\n\n");
        md.push_str("| File | Description |\n");
        md.push_str("|------|-------------|\n");
        md.push_str("| `subscription.txt` | Base64 of ALL configs |\n");
        md.push_str("| `working_subscription.txt` | Base64 of working configs only |\n\n");

        md.push_str("### Configuration Files\n\n");
        md.push_str("| File | Description |\n");
        md.push_str("|------|-------------|\n");
        md.push_str(&format!("| `all_configs.txt` | All {} generated configs |\n", output.all_configs.len()));
        md.push_str(&format!("| `config_links.txt` | {} working configs |\n", output.configs.len()));
        md.push_str("| `configs.json` | Complete JSON data |\n");
        md.push_str("| `vless_configs.txt` | VLESS protocol configs |\n");
        md.push_str("| `vmess_configs.txt` | VMess protocol configs |\n");
        md.push_str("| `trojan_configs.txt` | Trojan protocol configs |\n");
        md.push_str("| `shadowsocks_configs.txt` | Shadowsocks configs |\n");
        md.push_str("| `statistics.txt` | Detailed statistics |\n");
        md.push_str("| `qr_codes/` | QR codes for mobile setup |\n\n");

        // Show protocol breakdown
        md.push_str("## 📋 Protocol Breakdown\n\n");
        let protocols = vec!["VLESS", "VMess", "Trojan", "SS"];
        for protocol in protocols {
            let all_count = output.all_configs.iter().filter(|c| c.protocol == protocol).count();
            let working_count = output.configs.iter().filter(|c| c.protocol == protocol).count();
            if all_count > 0 {
                md.push_str(&format!("- **{}**: {} total, {} working\n", protocol, all_count, working_count));
            }
        }
        md.push_str("\n");

        // Show sample configs
        if !output.all_configs.is_empty() {
            md.push_str("## 🔗 Sample Configurations\n\n");
            
            // Show first few configs
            let sample_count = std::cmp::min(5, output.all_configs.len());
            md.push_str(&format!("### First {} configs:\n\n", sample_count));
            
            for (idx, config) in output.all_configs.iter().take(sample_count).enumerate() {
                let status = if config.is_working { "✅" } else { "⚠️" };
                let time = config.response_time_ms.map(|t| format!("{}ms", t)).unwrap_or_else(|| "N/A".to_string());
                md.push_str(&format!(
                    "{}. {} **{}** - {} | {}:{} | {}\n",
                    idx + 1,
                    status,
                    config.protocol,
                    config.transmission,
                    config.address,
                    config.port,
                    time
                ));
            }
            md.push_str("\n");
        }

        if output.all_configs.is_empty() {
            md.push_str("## ⚠️ Status\n\n");
            md.push_str("No proxy configurations are currently available.\n\n");
            md.push_str("**Possible Reasons:**\n");
            md.push_str("- No live proxies detected by scanner\n");
            md.push_str("- Scanner output file not found or empty\n\n");
            md.push_str("The system will automatically retry when new proxies are discovered.\n\n");
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

        // QR codes have size limits, truncate if needed
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
                "QR Code Directory\n=================\n\n\
                 No QR codes available yet.\n\
                 QR codes will be generated automatically when proxy configurations are created.\n",
            )?;
            println!("   ✓ qr_codes/README.txt (placeholder)");
            return Ok(());
        }

        let mut success_count = 0;
        let mut error_count = 0;

        // Generate QR codes for ALL configs, limit to first 100 to avoid too many files
        let max_qr_codes = std::cmp::min(100, configs.len());
        
        for (idx, config) in configs.iter().take(max_qr_codes).enumerate() {
            match self.create_qr_code(&config.link) {
                Ok(qr) => {
                    let status = if config.is_working { "working" } else { "untested" };
                    let filename = format!(
                        "qr_{:03}_{}_{}.txt",
                        idx + 1,
                        config.protocol.to_lowercase(),
                        status
                    );
                    let file_path = qr_dir.join(&filename);

                    let response_info = config
                        .response_time_ms
                        .map(|t| format!("{} ms", t))
                        .unwrap_or_else(|| "Not tested".to_string());

                    let status_emoji = if config.is_working { "✅ Working" } else { "⚠️ Untested/Failed" };

                    let content = format!(
                        "Config #{} - {} - {}\n\
                         ========================================\n\n\
                         Protocol: {}\n\
                         Transmission: {}\n\
                         Address: {}:{}\n\
                         Status: {}\n\
                         Response Time: {}\n\n\
                         QR Code:\n{}\n\n\
                         Configuration Link:\n{}",
                        idx + 1,
                        config.protocol,
                        config.transmission,
                        config.protocol,
                        config.transmission,
                        config.address,
                        config.port,
                        status_emoji,
                        response_info,
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

        // Create an index file
        let index_path = qr_dir.join("index.txt");
        let mut index_content = String::from("QR Code Index\n=============\n\n");
        index_content.push_str(&format!("Total QR codes: {}\n", success_count));
        index_content.push_str(&format!("Failed to generate: {}\n\n", error_count));
        index_content.push_str("Files:\n");
        
        if let Ok(entries) = fs::read_dir(&qr_dir) {
            let mut files: Vec<String> = entries
                .filter_map(|e| e.ok())
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|name| name.starts_with("qr_") && name.ends_with(".txt"))
                .collect();
            files.sort();
            for file in files {
                index_content.push_str(&format!("  - {}\n", file));
            }
        }
        
        fs::write(&index_path, index_content)?;

        println!(
            "   ✓ qr_codes/ ({} generated, {} errors)",
            success_count, error_count
        );
        Ok(())
    }
}
