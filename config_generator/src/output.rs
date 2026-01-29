use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::generator::{ConfigGenerator, ProxyConfig};
use crate::tester::{TestResult, TestStatistics};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigOutput {
    pub link: String,
    pub protocol: String,
    pub transmission: String,
    pub address: String,
    pub port: u16,
    pub is_working: bool,
    pub response_time_ms: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OutputBundle {
    pub all_configs: Vec<ConfigOutput>,
    pub working_configs: Vec<ConfigOutput>,
    pub subscription_base64: String,
    pub working_subscription_base64: String,
    pub statistics: TestStatistics,
    pub generated_at: String,
}

pub struct OutputGenerator {
    gen: ConfigGenerator,
}

impl Default for OutputGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputGenerator {
    pub fn new() -> Self {
        Self { gen: ConfigGenerator::new() }
    }

    pub fn create_bundle(&self, configs: &[ProxyConfig]) -> OutputBundle {
        let mut outputs = Vec::new();
        let mut links = Vec::new();

        for c in configs {
            let link = self.gen.to_link(c);
            links.push(link.clone());
            
            outputs.push(ConfigOutput {
                link,
                protocol: c.protocol.clone(),
                transmission: c.transmission.clone(),
                address: c.address.clone(),
                port: c.port,
                is_working: false,
                response_time_ms: None,
            });
        }

        let sub = general_purpose::STANDARD.encode(links.join("\n"));

        OutputBundle {
            all_configs: outputs,
            working_configs: Vec::new(),
            subscription_base64: sub,
            working_subscription_base64: String::new(),
            statistics: TestStatistics::default(),
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn create_bundle_with_results(&self, results: &[TestResult]) -> OutputBundle {
        let mut all = Vec::new();
        let mut working = Vec::new();
        let mut all_links = Vec::new();
        let mut working_links = Vec::new();

        for r in results {
            let link = self.gen.to_link(&r.config);
            all_links.push(link.clone());

            let output = ConfigOutput {
                link: link.clone(),
                protocol: r.config.protocol.clone(),
                transmission: r.config.transmission.clone(),
                address: r.config.address.clone(),
                port: r.config.port,
                is_working: r.is_working,
                response_time_ms: r.response_time_ms,
            };

            all.push(output.clone());
            
            if r.is_working {
                working_links.push(link);
                working.push(output);
            }
        }

        OutputBundle {
            all_configs: all,
            working_configs: working,
            subscription_base64: general_purpose::STANDARD.encode(all_links.join("\n")),
            working_subscription_base64: general_purpose::STANDARD.encode(working_links.join("\n")),
            statistics: TestStatistics::from_results(results),
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn save_empty_files(&self, dir: &str) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(dir)?;
        fs::create_dir_all(format!("{}/qr_codes", dir))?;

        fs::write(Path::new(dir).join("all_configs.txt"), "# No PROXY LIVE IPs found in scanner output")?;
        fs::write(Path::new(dir).join("config_links.txt"), "# No configs available")?;
        fs::write(Path::new(dir).join("subscription.txt"), "")?;
        fs::write(Path::new(dir).join("vless_configs.txt"), "# No VLESS configs")?;
        fs::write(Path::new(dir).join("vmess_configs.txt"), "# No VMess configs")?;
        fs::write(Path::new(dir).join("trojan_configs.txt"), "# No Trojan configs")?;
        fs::write(Path::new(dir).join("statistics.txt"), "No data - scanner found no PROXY LIVE IPs")?;
        fs::write(Path::new(dir).join("configs.json"), "{}")?;
        fs::write(Path::new(dir).join("qr_codes/README.txt"), "No QR codes - no configs generated")?;

        println!("   ✓ Empty placeholder files created");
        Ok(())
    }

    pub fn save_all_files(&self, bundle: &OutputBundle, dir: &str) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(dir)?;
        fs::create_dir_all(format!("{}/qr_codes", dir))?;

        // all_configs.txt - ALL generated configs
        let all_links: Vec<&str> = bundle.all_configs.iter().map(|c| c.link.as_str()).collect();
        fs::write(Path::new(dir).join("all_configs.txt"), all_links.join("\n\n"))?;
        println!("   ✓ all_configs.txt ({} configs)", all_links.len());

        // config_links.txt - Working only
        let working_links: Vec<&str> = bundle.working_configs.iter().map(|c| c.link.as_str()).collect();
        if working_links.is_empty() {
            fs::write(Path::new(dir).join("config_links.txt"), 
                "# No working configs yet\n# See all_configs.txt for all generated configs")?;
        } else {
            fs::write(Path::new(dir).join("config_links.txt"), working_links.join("\n\n"))?;
        }
        println!("   ✓ config_links.txt ({} working)", working_links.len());

        // subscription.txt (Base64)
        fs::write(Path::new(dir).join("subscription.txt"), &bundle.subscription_base64)?;
        println!("   ✓ subscription.txt");

        // working_subscription.txt
        fs::write(Path::new(dir).join("working_subscription.txt"), &bundle.working_subscription_base64)?;
        println!("   ✓ working_subscription.txt");

        // Protocol-specific files
        self.save_by_protocol(bundle, dir)?;

        // statistics.txt
        let stats = self.format_statistics(bundle);
        fs::write(Path::new(dir).join("statistics.txt"), stats)?;
        println!("   ✓ statistics.txt");

        // configs.json
        let json = serde_json::to_string_pretty(bundle)?;
        fs::write(Path::new(dir).join("configs.json"), json)?;
        println!("   ✓ configs.json");

        // README.md
        self.save_readme(bundle, dir)?;

        Ok(())
    }

    fn save_by_protocol(&self, bundle: &OutputBundle, dir: &str) -> Result<(), Box<dyn std::error::Error>> {
        let protocols = [("vless", "VLESS"), ("vmess", "VMess"), ("trojan", "Trojan")];

        for (prefix, name) in protocols {
            let links: Vec<&str> = bundle.all_configs.iter()
                .filter(|c| c.protocol == prefix)
                .map(|c| c.link.as_str())
                .collect();

            let filename = format!("{}_configs.txt", prefix);
            let content = if links.is_empty() {
                format!("# No {} configs", name)
            } else {
                links.join("\n\n")
            };

            fs::write(Path::new(dir).join(&filename), content)?;
            println!("   ✓ {} ({} configs)", filename, links.len());
        }

        Ok(())
    }

    fn format_statistics(&self, bundle: &OutputBundle) -> String {
        format!(
            "Proxy Configuration Statistics\n\
             ===============================\n\n\
             Generated: {}\n\n\
             Total Configs: {}\n\
             Working Configs: {}\n\
             Failed Configs: {}\n\
             Success Rate: {:.1}%\n\n\
             Response Times:\n\
             - Average: {:.0} ms\n\
             - Fastest: {} ms\n\
             - Slowest: {} ms\n\n\
             By Protocol:\n\
             - VLESS: {}\n\
             - VMess: {}\n\
             - Trojan: {}\n",
            bundle.generated_at,
            bundle.all_configs.len(),
            bundle.statistics.working_configs,
            bundle.statistics.failed_configs,
            bundle.statistics.success_rate,
            bundle.statistics.average_response_time_ms,
            bundle.statistics.fastest_response_time_ms.map(|t| t.to_string()).unwrap_or("N/A".into()),
            bundle.statistics.slowest_response_time_ms.map(|t| t.to_string()).unwrap_or("N/A".into()),
            bundle.all_configs.iter().filter(|c| c.protocol == "vless").count(),
            bundle.all_configs.iter().filter(|c| c.protocol == "vmess").count(),
            bundle.all_configs.iter().filter(|c| c.protocol == "trojan").count(),
        )
    }

    fn save_readme(&self, bundle: &OutputBundle, dir: &str) -> Result<(), Box<dyn std::error::Error>> {
        let md = format!(
            "# Proxy Configurations\n\n\
             **Generated:** {}\n\n\
             ## Statistics\n\n\
             | Metric | Value |\n\
             |--------|-------|\n\
             | Total Configs | {} |\n\
             | Working | {} |\n\
             | Success Rate | {:.1}% |\n\n\
             ## Files\n\n\
             - `all_configs.txt` - All {} configs\n\
             - `config_links.txt` - {} working configs\n\
             - `subscription.txt` - Base64 subscription\n\
             - `vless_configs.txt` - VLESS only\n\
             - `vmess_configs.txt` - VMess only\n\
             - `trojan_configs.txt` - Trojan only\n\
             - `qr_codes/` - QR codes\n\n\
             ---\n\
             *Auto-generated from PROXY LIVE IPs*\n",
            bundle.generated_at,
            bundle.all_configs.len(),
            bundle.statistics.working_configs,
            bundle.statistics.success_rate,
            bundle.all_configs.len(),
            bundle.working_configs.len(),
        );

        fs::write(Path::new(dir).join("README.md"), md)?;
        println!("   ✓ README.md");
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
    pub fn new() -> Self { Self }

    pub fn generate_qr_codes(&self, configs: &[ConfigOutput], dir: &str) -> Result<(), Box<dyn std::error::Error>> {
        let qr_dir = Path::new(dir).join("qr_codes");
        fs::create_dir_all(&qr_dir)?;

        if configs.is_empty() {
            fs::write(qr_dir.join("README.txt"), "No configs for QR codes")?;
            println!("   ✓ qr_codes/ (empty)");
            return Ok(());
        }

        let mut count = 0;
        let max = std::cmp::min(50, configs.len());

        for (i, c) in configs.iter().take(max).enumerate() {
            if let Ok(qr) = self.make_qr(&c.link) {
                let name = format!("qr_{:03}_{}.txt", i + 1, c.protocol);
                let content = format!(
                    "{} - {} - {}:{}\n{}\n\n{}\n\nLink:\n{}",
                    c.protocol.to_uppercase(), c.transmission, c.address, c.port,
                    "=".repeat(40), qr, c.link
                );
                
                if fs::write(qr_dir.join(&name), content).is_ok() {
                    count += 1;
                }
            }
        }

        fs::write(qr_dir.join("index.txt"), format!("Generated: {} QR codes", count))?;
        println!("   ✓ qr_codes/ ({} files)", count);
        Ok(())
    }

    fn make_qr(&self, data: &str) -> Result<String, String> {
        use qrcode::render::unicode;
        use qrcode::QrCode;

        let input = if data.len() > 2900 { &data[..2900] } else { data };
        let code = QrCode::new(input.as_bytes()).map_err(|e| e.to_string())?;
        
        Ok(code.render::<unicode::Dense1x2>()
            .dark_color(unicode::Dense1x2::Light)
            .light_color(unicode::Dense1x2::Dark)
            .build())
    }
}
