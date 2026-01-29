use std::fs;
use std::path::Path;

mod generator;
mod output;
mod tester;

use generator::ConfigGenerator;
use output::{OutputGenerator, SubscriptionManager};
use tester::ConfigTester;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Advanced Proxy Configuration Generator");
    println!("==========================================\n");

    let output_dir = "sub/configs";

    // Create directories
    fs::create_dir_all(output_dir)?;
    fs::create_dir_all(format!("{}/qr_codes", output_dir))?;
    println!("📁 Output directory: {}\n", output_dir);

    // Read proxies
    let proxies = read_proxy_list("sub/ProxyIP-Daily.md")?;
    println!("📡 Found {} live proxies from scanner\n", proxies.len());

    let config_generator = ConfigGenerator::new();
    let output_generator = OutputGenerator::new();
    let subscription_manager = SubscriptionManager::new(None);

    if proxies.is_empty() {
        println!("⚠️  No proxies found. Creating empty output files...\n");

        let empty_bundle = output_generator.create_empty_output();

        println!("📝 Saving files:");
        output_generator.save_to_files(&empty_bundle, output_dir)?;
        subscription_manager.save_qr_codes(&empty_bundle.configs, output_dir)?;

        println!("\n✅ Empty output files created successfully!");
        println!("📝 Note: Files will be populated once live proxies are detected.\n");

        list_output_files(output_dir);
        return Ok(());
    }

    // Generate configs
    println!("⚙️  Generating configurations...");
    let mut all_configs = Vec::new();

    for proxy in &proxies {
        let configs = config_generator.generate_configs(&proxy.ip, proxy.port);
        all_configs.extend(configs);
    }

    println!("✅ Generated {} total configurations\n", all_configs.len());

    // Test configs
    println!("🔬 Testing configurations...");
    let config_tester = ConfigTester::new(10, 50);
    let test_results = config_tester.test_configs(all_configs).await;

    let working_count = test_results.iter().filter(|r| r.is_working).count();
    println!(
        "✅ Testing complete: {}/{} configs are working\n",
        working_count,
        test_results.len()
    );

    // Generate output
    println!("📝 Generating output files:");
    let output_bundle = output_generator.generate_output(test_results);

    output_generator.save_to_files(&output_bundle, output_dir)?;

    println!("\n📱 Generating QR codes:");
    subscription_manager.save_qr_codes(&output_bundle.configs, output_dir)?;

    println!("\n✨ Success! Output saved to: {}/", output_dir);
    println!("\n📊 Summary:");
    println!(
        "   Total Configs: {}",
        output_bundle.statistics.total_configs
    );
    println!("   Working: {}", output_bundle.statistics.working_configs);
    println!("   Failed: {}", output_bundle.statistics.failed_configs);
    println!(
        "   Success Rate: {:.2}%",
        output_bundle.statistics.success_rate
    );
    println!(
        "   Avg Response: {:.2} ms",
        output_bundle.statistics.average_response_time_ms
    );

    println!();
    list_output_files(output_dir);

    Ok(())
}

fn list_output_files(output_dir: &str) {
    println!("📂 Output files:");

    let files = vec![
        "subscription.txt",
        "configs.json",
        "config_links.txt",
        "vless_configs.txt",
        "vmess_configs.txt",
        "trojan_configs.txt",
        "shadowsocks_configs.txt",
        "statistics.txt",
        "README.md",
    ];

    for file in files {
        let path = Path::new(output_dir).join(file);
        if path.exists() {
            if let Ok(meta) = fs::metadata(&path) {
                println!("   ✓ {} ({} bytes)", file, meta.len());
            } else {
                println!("   ✓ {}", file);
            }
        } else {
            println!("   ✗ {} (missing)", file);
        }
    }

    let qr_dir = Path::new(output_dir).join("qr_codes");
    if qr_dir.exists() {
        if let Ok(entries) = fs::read_dir(&qr_dir) {
            let count = entries.count();
            println!("   ✓ qr_codes/ ({} files)", count);
        }
    }
}

#[derive(Debug)]
struct ProxyInfo {
    ip: String,
    port: u16,
    #[allow(dead_code)]
    location: String,
    #[allow(dead_code)]
    response_time: u32,
}

fn read_proxy_list(file_path: &str) -> Result<Vec<ProxyInfo>, Box<dyn std::error::Error>> {
    if !Path::new(file_path).exists() {
        println!("⚠️  Scanner output file not found: {}", file_path);
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(file_path)?;

    if content.trim().is_empty() || content.len() < 50 {
        println!("⚠️  Scanner output file is empty or too small");
        return Ok(Vec::new());
    }

    let mut proxies = Vec::new();

    println!("📄 Reading scanner output...");

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip non-proxy lines
        if trimmed.is_empty()
            || trimmed.starts_with('<')
            || trimmed.starts_with('>')
            || trimmed.starts_with('#')
            || trimmed.starts_with('|')
            || trimmed.starts_with('[')
            || trimmed.starts_with('*')
            || trimmed.starts_with('-')
        {
            continue;
        }

        if line.contains("PROXY LIVE") {
            if let Some(proxy) = parse_proxy_line(line) {
                proxies.push(proxy);
            }
        }
    }

    println!("✅ Parsed {} proxy entries\n", proxies.len());
    Ok(proxies)
}

fn parse_proxy_line(line: &str) -> Option<ProxyInfo> {
    let colon_pos = line.find(':')?;
    let after_colon = &line[colon_pos + 1..];

    let parts: Vec<&str> = after_colon.split_whitespace().collect();

    for part in &parts {
        if part.contains('✅') || part.contains("ms") || part.contains('(') || part.contains(')')
        {
            continue;
        }

        let ip_candidate = part
            .trim()
            .trim_matches(|c: char| !c.is_numeric() && c != '.');

        if validate_ip(ip_candidate) {
            let response_time = extract_response_time(line).unwrap_or(0);
            let location = extract_location(line);

            return Some(ProxyInfo {
                ip: ip_candidate.to_string(),
                port: 443,
                location,
                response_time,
            });
        }
    }

    None
}

fn extract_response_time(text: &str) -> Option<u32> {
    if let Some(start) = text.find('(') {
        if let Some(end) = text[start..].find("ms") {
            let num_part = &text[start + 1..start + end];
            let num_str: String = num_part.chars().filter(|c| c.is_numeric()).collect();
            return num_str.parse().ok();
        }
    }
    None
}

fn extract_location(text: &str) -> String {
    if let Some(dash_pos) = text.rfind('-') {
        let location = text[dash_pos + 1..].trim();
        let clean: String = location
            .chars()
            .take_while(|c| c.is_alphabetic() || c.is_whitespace())
            .collect();
        if !clean.is_empty() {
            return clean.trim().to_string();
        }
    }
    "Unknown".to_string()
}

fn validate_ip(ip: &str) -> bool {
    let parts: Vec<&str> = ip.split('.').collect();

    if parts.len() != 4 {
        return false;
    }

    parts.iter().all(|part| part.parse::<u8>().is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_validation() {
        assert!(validate_ip("192.168.1.1"));
        assert!(validate_ip("8.8.8.8"));
        assert!(!validate_ip("256.1.1.1"));
        assert!(!validate_ip("192.168.1"));
    }

    #[test]
    fn test_proxy_parsing() {
        let line = "PROXY LIVE ✅: 167.114.67.25 (989 ms) - Beauharnois";
        let proxy = parse_proxy_line(line).unwrap();
        assert_eq!(proxy.ip, "167.114.67.25");
        assert_eq!(proxy.response_time, 989);
    }

    #[test]
    fn test_response_time() {
        assert_eq!(extract_response_time("test (123 ms) end"), Some(123));
    }
}
