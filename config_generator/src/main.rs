use std::fs;

mod generator;
mod tester;
mod output;

use generator::ConfigGenerator;
use tester::ConfigTester;
use output::{OutputGenerator, SubscriptionManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Advanced Proxy Configuration Generator");
    println!("==========================================\n");

    // Read proxy IPs from the scanner output
    let proxies = read_proxy_list("sub/ProxyIP-Daily.md")?;
    println!("📡 Found {} live proxies from scanner\n", proxies.len());

    if proxies.is_empty() {
        println!("⚠️  No proxies found. Exiting.");
        return Ok(());
    }

    // Initialize components
    let config_generator = ConfigGenerator::new();
    let config_tester = ConfigTester::new(10, 50); // 10 sec timeout, 50 concurrent
    let output_generator = OutputGenerator::new();
    let subscription_manager = SubscriptionManager::new(None);

    // Generate configurations for all proxies
    println!("⚙️  Generating configurations...");
    let mut all_configs = Vec::new();
    
    for proxy in &proxies {
        let configs = config_generator.generate_configs(&proxy.ip, proxy.port);
        all_configs.extend(configs);
    }

    println!("✅ Generated {} total configurations\n", all_configs.len());

    // Test all configurations
    println!("🔍 Testing configurations (this may take a while)...");
    let test_results = config_tester.test_configs(all_configs).await;
    
    let working_count = test_results.iter().filter(|r| r.is_working).count();
    println!("✅ Testing complete: {}/{} configs are working\n", 
             working_count, test_results.len());

    // Generate output
    println!("📝 Generating output files...");
    let output_bundle = output_generator.generate_output(test_results);
    
    // Save to files
    let output_dir = "sub/configs";
    output_generator.save_to_files(&output_bundle, output_dir)?;
    
    // Generate QR codes
    println!("📱 Generating QR codes...");
    subscription_manager.save_qr_codes(&output_bundle.configs, output_dir)?;

    println!("\n✨ Success! Output saved to: {}/", output_dir);
    println!("\n📊 Summary:");
    println!("   Total Configs: {}", output_bundle.statistics.total_configs);
    println!("   Working: {}", output_bundle.statistics.working_configs);
    println!("   Failed: {}", output_bundle.statistics.failed_configs);
    println!("   Success Rate: {:.2}%", output_bundle.statistics.success_rate);
    println!("   Avg Response: {:.2} ms", output_bundle.statistics.average_response_time_ms);

    Ok(())
}

#[derive(Debug)]
struct ProxyInfo {
    ip: String,
    port: u16,
    location: String,
    response_time: u32,
}

fn read_proxy_list(file_path: &str) -> Result<Vec<ProxyInfo>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(file_path)?;
    let mut proxies = Vec::new();

    for line in content.lines() {
        // Parse lines like: "PROXY LIVE ✅: 167.114.67.25 (989 ms) - Beauharnois"
        if line.contains("PROXY LIVE") && line.contains("✅") {
            if let Some(proxy) = parse_proxy_line(line) {
                proxies.push(proxy);
            }
        }
    }

    Ok(proxies)
}

fn parse_proxy_line(line: &str) -> Option<ProxyInfo> {
    // Extract IP address
    let parts: Vec<&str> = line.split_whitespace().collect();
    
    for (_i, part) in parts.iter().enumerate() {
        // Find IP address (format: xxx.xxx.xxx.xxx)
        if part.contains('.') && part.split('.').count() == 4 {
            let ip = part.trim_matches(|c: char| !c.is_numeric() && c != '.').to_string();
            
            // Validate IP format
            if validate_ip(&ip) {
                // Extract response time
                let response_time = parts
                    .iter()
                    .find(|p| p.contains("ms"))
                    .and_then(|p| {
                        p.trim_matches(|c: char| !c.is_numeric())
                            .parse::<u32>()
                            .ok()
                    })
                    .unwrap_or(0);

                // Extract location (last part after -)
                let location = parts
                    .iter()
                    .skip_while(|p| !p.contains('-'))
                    .skip(1)
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
                    .join(" ");

                return Some(ProxyInfo {
                    ip,
                    port: 443, // From your scanner config
                    location: location.trim().to_string(),
                    response_time,
                });
            }
        }
    }

    None
}

fn validate_ip(ip: &str) -> bool {
    let parts: Vec<&str> = ip.split('.').collect();
    
    if parts.len() != 4 {
        return false;
    }

    parts.iter().all(|part| {
        part.parse::<u8>().is_ok()
    })
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
        assert!(!validate_ip("invalid"));
    }

    #[test]
    fn test_proxy_parsing() {
        let line = "PROXY LIVE ✅: 167.114.67.25 (989 ms) - Beauharnois";
        let proxy = parse_proxy_line(line);
        
        assert!(proxy.is_some());
        let proxy = proxy.unwrap();
        assert_eq!(proxy.ip, "167.114.67.25");
        assert_eq!(proxy.port, 443);
        assert_eq!(proxy.response_time, 989);
        assert_eq!(proxy.location, "Beauharnois");
    }
}
