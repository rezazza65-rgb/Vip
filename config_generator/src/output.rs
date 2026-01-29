use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use std::fs;
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

    pub fn create_bundle_from_configs(&self, configs: &[ProxyConfig]) -> OutputBundle {
        let timestamp = chrono::Utc::now().to_rfc3339();
        let mut all_outputs = Vec::new();
        let mut all_links = Vec::new();

        for config in configs {
            let link = self.generator.to_link(config);
            all_links.push(link.clone());

            all_outputs.push(ConfigOutput {
                link,
                protocol: config.protocol.to_string(),
                transmission: config.transmission.to_string(),
                address: config.address.clone(),
                port: config.port,
                is_tested: false,
                is_working: false,
                response_time_ms: None,
            });
        }

        let sub_content = all_links.join("\n");
        let subscription_link = general_purpose::STANDARD.encode(&sub_content);

        OutputBundle {
            subscription_link: subscription_link.clone(),
            working_subscription_link: String::new(),
            all_configs: all_outputs,
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

    pub fn create_bundle_from_results(&self, results: Vec<TestResult>) -> OutputBundle {
        let timestamp = chrono::Utc::now().to_rfc3339();
        
        let mut all_outputs = Vec::new();
        let mut working_outputs = Vec::new();
        let mut all_links = Vec::new();
        let mut working_links = Vec::new();

        for result in &results {
            let link = self.generator.to_link(&result.config);
            all_links.push(link.clone());

            let output = ConfigOutput {
                link: link.clone(),
                protocol: result.config.protocol.to_string(),
                transmission: result.config.transmission.to_string(),
                address: result.config.address.clone(),
                port: result.config.port,
                is_tested: true,
                is_working: result.is_working,
                response_time_ms: result.response_time_ms,
            };

            all_outputs.push(output.clone());

            if result.is_working {
                working_links.push(link);
                working_outputs.push(output);
            }
        }

        let sub_content = all_links.join("\n");
        let subscription_link = general_purpose::STANDARD.encode(&sub_content);

        let working_content = working_links.join("\n");
        let working_subscription_link = general_purpose::STANDARD.encode(&working_content);

        let statistics = TestStatistics::from_results(&results);

        OutputBundle {
            subscription_link,
            working_subscription_link,
            all_configs: all_outputs,
            configs: working_outputs,
            statistics,
            generated_at: timestamp,
        }
    }

    pub fn save_to_files(&self, bundle: &OutputBundle, output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(output_dir)?;
        fs::create_dir_all(format!("{}/qr_codes", output_dir))?;

        // subscription.txt
        fs::write(
            Path::new(output_dir).join("subscription.txt"),
            &bundle.subscription_link
        )?;
        println!("      ✓ subscription.txt");

        // working_subscription.txt
        fs::write(
            Path::new(output_dir).join("working_subscription.txt"),
            &bundle.working_subscription_link
        )?;
        println!("      ✓ working_subscription.txt");

        // configs.json
        let json = serde_json::to_string_pretty(bundle)?;
        fs::write(Path::new(output_dir).join("configs.json"), json)?;
        println!("      ✓ configs.json");

        // all_configs.txt - ALL configs (most important!)
        let all_links: Vec<String> = bundle.all_configs.iter().map(|c| c.link.clone()).collect();
        fs::write(
            Path::new(output_dir).join("all_configs.txt"),
            all_links.join("\n\n")
        )?;
        println!("      ✓ all_configs.txt ({} configs)", all_links.len());

        // config_links.txt - working only
        let working_links: Vec<String> = bundle.configs.iter().map(|c| c.link.clone()).collect();
        if working_links.is_empty() {
            fs::write(
                Path::new(output_dir).join("config_links.txt"),
                "# No working configs - see all_configs.txt for all generated configs"
            )?;
        } else {
            fs::write(
                Path::new(output_dir).join("config_links.txt"),
                working_links.join("\n\n")
            )?;
        }
        println!("      ✓ config_links.txt ({} working)", working_links.len());

        // statistics.txt
        let stats = format!(
            "Configuration Statistics\n\
             ========================\n\n\
             Generated: {}\n\n\
             Total Configs: {}\n\
             Working: {}\n\
             Failed: {}\n\
             Success Rate: {:.1}%\n\n\
             Response Times:\n\
             - Average: {:.0} ms\n\
             - Fastest: {} ms\n\
             - Slowest: {} ms\n",
            bundle.generated_at,
            bundle.all_configs.len(),
            bundle.statistics.working_configs,
            bundle.statistics.failed_configs,
            bundle.statistics.success_rate,
            bundle.statistics.average_response_time_ms,
            bundle.statistics.fastest_response_time_ms.map(|t| t.to_string()).unwrap_or_else(|| "N/A".to_string()),
            bundle.statistics.slowest_response_time_ms.map(|t| t.to_string()).unwrap_or_else(|| "N/A".to_string()),
        );
        fs::write(Path::new(output_dir).join("statistics.txt"), stats)?;
        println!("      ✓ statistics.txt");

        // Protocol-specific files
        self.save_by_protocol(bundle, output_dir)?;

        // README.md
        self.save_readme(bundle, output_dir)?;

        Ok(())
    }

    fn save_by_protocol(&self, bundle: &OutputBundle, output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
        let protocols = vec![
            ("VLESS", "vless"),
            ("VMess", "vmess"),
            ("Trojan", "trojan"),
            ("SS", "shadowsocks"),
        ];

        for (name, prefix) in protocols {
            let configs: Vec<String> = bundle.all_configs.iter()
                .filter(|c| c.protocol == name)
                .map(|c| c.link.clone())
                .collect();

            let filename = format!("{}_configs.txt", prefix);
            let content = if configs.is_empty() {
                format!("# No {} configs generated", name)
            } else {
                configs.join("\n\n")
            };

            fs::write(Path::new(output_dir).join(&filename), content)?;
            println!("      ✓ {} ({} configs)", filename, configs.len());
        }

        Ok(())
    }

    fn save_readme(&self, bundle: &OutputBundle, output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut md = String::new();
        
        md.push_str("# Proxy Configurations\n\n");
        md.push_str(&format!("**Generated:** {}\n\n", bundle.generated_at));
        
        md.push_str("## Statistics\n\n");
        md.push_str(&format!("- **Total Configs:** {}\n", bundle.all_configs.len()));
        md.push_str(&format!("- **Working:** {}\n", bundle.statistics.working_configs));
        md.push_str(&format!("- **Success Rate:** {:.1}%\n\n", bundle.statistics.success_rate));
        
        md.push_str("## Files\n\n");
        md.push_str("| File | Description |\n");
        md.push_str("|------|-------------|\n");
        md.push_str(&format!("| `all_configs.txt` | All {} configs |\n", bundle.all_configs.len()));
        md.push_str(&format!("| `config_links.txt` | {} working configs |\n", bundle.configs.len()));
        md.push_str("| `subscription.txt` | Base64 subscription |\n");
        md.push_str("| `vless_configs.txt` | VLESS only |\n");
        md.push_str("| `vmess_configs.txt` | VMess only |\n");
        md.push_str("| `trojan_configs.txt` | Trojan only |\n");
        md.push_str("| `shadowsocks_configs.txt` | SS only |\n");
        md.push_str("| `qr_codes/` | QR codes |\n\n");

        md.push_str("## Protocol Breakdown\n\n");
        for protocol in &["VLESS", "VMess", "Trojan", "SS"] {
            let count = bundle.all_configs.iter().filter(|c| c.protocol == *protocol).count();
            let working = bundle.configs.iter().filter(|c| c.protocol == *protocol).count();
            md.push_str(&format!("- **{}:** {} total, {} working\n", protocol, count, working));
        }

        md.push_str("\n---\n*Auto-generated*\n");

        fs::write(Path::new(output_dir).join("README.md"), md)?;
        println!("      ✓ README.md");

        Ok(())
    }
}

pub struct SubscriptionManager;

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self
    }

    pub fn save_qr_codes(&self, configs: &[ConfigOutput], output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
        let qr_dir = Path::new(output_dir).join("qr_codes");
        fs::create_dir_all(&qr_dir)?;

        if configs.is_empty() {
            fs::write(qr_dir.join("README.txt"), "No configs available for QR codes")?;
            return Ok(());
        }

        let mut success = 0;
        let max_qr = std::cmp::min(50, configs.len());

        for (i, config) in configs.iter().take(max_qr).enumerate() {
            if let Ok(qr) = self.create_qr(&config.link) {
                let status = if config.is_working { "working" } else { "generated" };
                let filename = format!("qr_{:03}_{}_{}.txt", i + 1, config.protocol.to_lowercase(), status);
                
                let content = format!(
                    "Config #{} - {} - {}\n{}\n\n{}\n\nLink:\n{}",
                    i + 1,
                    config.protocol,
                    config.transmission,
                    "=".repeat(40),
                    qr,
                    config.link
                );

                if fs::write(qr_dir.join(&filename), content).is_ok() {
                    success += 1;
                }
            }
        }

        // Index file
        fs::write(
            qr_dir.join("index.txt"),
            format!("QR Codes Generated: {}\nTotal Configs: {}", success, configs.len())
        )?;

        println!("      ✓ qr_codes/ ({} files)", success);
        Ok(())
    }

    fn create_qr(&self, link: &str) -> Result<String, String> {
        use qrcode::render::unicode;
        use qrcode::QrCode;

        let data = if link.len() > 2900 { &link[..2900] } else { link };
        
        let code = QrCode::new(data.as_bytes()).map_err(|e| e.to_string())?;
        
        Ok(code.render::<unicode::Dense1x2>()
            .dark_color(unicode::Dense1x2::Light)
            .light_color(unicode::Dense1x2::Dark)
            .build())
    }
}
