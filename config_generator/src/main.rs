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

    // Create directories with proper error handling
    if let Err(e) = fs::create_dir_all(output_dir) {
        eprintln!("❌ Failed to create output directory: {}", e);
        return Err(e.into());
    }
    if let Err(e) = fs::create_dir_all(format!("{}/qr_codes", output_dir)) {
        eprintln!("❌ Failed to create qr_codes directory: {}", e);
        return Err(e.into());
    }
    println!("📁 Output directory: {}\n", output_dir);

    // Read proxies with detailed error handling
    let proxies = match read_proxy_list("sub/ProxyIP-Daily.md") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("⚠️ Error reading proxy list: {}", e);
            Vec::new()
        }
    };
    println!("📡 Found {} live proxies from scanner\n", proxies.len());

    let config_generator = ConfigGenerator::new();
    let output_generator = OutputGenerator::new();
    let subscription_manager = SubscriptionManager::new(None);

    if proxies.is_empty() {
        println!("⚠️  No proxies found. Creating empty output files...\n");

        let empty_bundle = output_generator.create_empty_output();

        println!("📝 Saving files:");
        if let Err(e) = output_generator.save_to_files(&empty_bundle, output_dir) {
            eprintln!("❌ Failed to save files: {}", e);
        }
        if let Err(e) = subscription_manager.save_qr_codes(&empty_bundle.all_configs, output_dir) {
            eprintln!("❌ Failed to save QR codes: {}", e);
        }

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

    let total_generated = all_configs.len();
    println!("✅ Generated {} total configurations\n", total_generated);

    // Save generated configs BEFORE testing (in case testing fails)
    println!("📝 Saving generated configs before testing...");
    let pre_test_bundle = output_generator.generate_output_without_testing(&all_configs);
    if let Err(e) = output_generator.save_all_generated_configs(&pre_test_bundle, output_dir) {
        eprintln!("⚠️ Warning: Could not save pre-test configs: {}", e);
    }

    // Test configs
    println!("\n🔬 Testing configurations...");
    let config_tester = ConfigTester::new(10, 50);
    let test_results = config_tester.test_configs(all_configs).await;

    let working_count = test_results.iter().filter(|r| r.is_working).count();
    let failed_count = test_results.len() - working_count;
    println!(
        "✅ Testing complete: {}/{} configs are working ({} failed)\n",
        working_count,
        test_results.len(),
        failed_count
    );

    // Generate output with ALL configs (both working and failed)
    println!("📝 Generating output files:");
    let output_bundle = output_generator.generate_output(test_results);

    if let Err(e) = output_generator.save_to_files(&output_bundle, output_dir) {
        eprintln!("❌ Failed to save output files: {}", e);
    }

    println!("\n📱 Generating QR codes:");
    // Generate QR codes for ALL configs, not just working ones
    if let Err(e) = subscription_manager.save_qr_codes(&output_bundle.all_configs, output_dir) {
        eprintln!("❌ Failed to save QR codes: {}", e);
    }

    println!("\n✨ Success! Output saved to: {}/", output_dir);
    println!("\n📊 Summary:");
    println!(
        "   Total Generated: {}",
        output_bundle.statistics.total_configs
    );
    println!("   Working: {}", output_bundle.statistics.working_configs);
    println!("   Failed: {}", output_bundle.statistics.failed_configs);
    println!(
        "   Success Rate: {:.2}%",
        output_bundle.statistics.success_rate
    );
    if output_bundle.statistics.average_response_time_ms > 0.0 {
        println!(
            "   Avg Response: {:.2} ms",
            output_bundle.statistics.average_response_time_ms
        );
    }

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
        "all_configs.txt",
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

#[derive(Debug, Clone)]
pub struct ProxyInfo {
    pub ip: String,
    pub port: u16,
    #[allow(dead_code)]
    pub location: String,
    #[allow(dead_code)]
    pub response_time: u32,
}

fn read_proxy_list(file_path: &str) -> Result<Vec<ProxyInfo>, Box<dyn std::error::Error>> {
    println!("📄 Reading scanner output from: {}", file_path);
    
    if !Path::new(file_path).exists() {
        println!("⚠️  Scanner output file not found: {}", file_path);
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(file_path)?;
    println!("📄 File size: {} bytes", content.len());

    if content.trim().is_empty() {
        println!("⚠️  Scanner output file is empty");
        return Ok(Vec::new());
    }

    let mut proxies = Vec::new();
    let mut lines_processed = 0;
    let mut proxy_lines_found = 0;

    for line in content.lines() {
        lines_processed += 1;
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
            || trimmed.starts_with("```")
        {
            continue;
        }

        // Look for various proxy line formats
        if line.contains("PROXY LIVE") || line.contains("✅") {
            proxy_lines_found += 1;
            if let Some(proxy) = parse_proxy_line(line) {
                proxies.push(proxy);
            } else {
                println!("   ⚠️ Could not parse: {}", line.chars().take(60).collect::<String>());
            }
        } else if let Some(proxy) = try_parse_ip_line(line) {
            // Try to parse any line that might contain an IP
            proxy_lines_found += 1;
            proxies.push(proxy);
        }
    }

    println!("📊 Processed {} lines, found {} proxy lines, parsed {} proxies", 
             lines_processed, proxy_lines_found, proxies.len());
    
    if proxies.is_empty() && proxy_lines_found > 0 {
        println!("⚠️ Found proxy lines but couldn't parse them. Sample lines from file:");
        for (i, line) in content.lines().take(10).enumerate() {
            println!("   Line {}: {}", i + 1, line.chars().take(80).collect::<String>());
        }
    }

    Ok(proxies)
}

fn parse_proxy_line(line: &str) -> Option<ProxyInfo> {
    // Try multiple parsing strategies
    
    // Strategy 1: Look for IP after colon
    if let Some(colon_pos) = line.find(':') {
        let after_colon = &line[colon_pos + 1..];
        if let Some(ip) = extract_ip_from_text(after_colon) {
            let response_time = extract_response_time(line).unwrap_or(0);
            let location = extract_location(line);
            return Some(ProxyInfo {
                ip,
                port: 443,
                location,
                response_time,
            });
        }
    }
    
    // Strategy 2: Look for IP anywhere in the line
    if let Some(ip) = extract_ip_from_text(line) {
        let response_time = extract_response_time(line).unwrap_or(0);
        let location = extract_location(line);
        return Some(ProxyInfo {
            ip,
            port: 443,
            location,
            response_time,
        });
    }

    None
}

fn try_parse_ip_line(line: &str) -> Option<ProxyInfo> {
    if let Some(ip) = extract_ip_from_text(line) {
        return Some(ProxyInfo {
            ip,
            port: 443,
            location: extract_location(line),
            response_time: extract_response_time(line).unwrap_or(0),
        });
    }
    None
}

fn extract_ip_from_text(text: &str) -> Option<String> {
    // Use regex-like pattern matching to find IP addresses
    let parts: Vec<&str> = text.split_whitespace().collect();
    
    for part in &parts {
        // Clean the part from non-IP characters
        let cleaned: String = part
            .chars()
            .filter(|c| c.is_numeric() || *c == '.')
            .collect();
        
        if validate_ip(&cleaned) {
            return Some(cleaned);
        }
        
        // Also try the raw part after trimming
        let trimmed = part.trim_matches(|c: char| !c.is_numeric() && c != '.');
        if validate_ip(trimmed) {
            return Some(trimmed.to_string());
        }
    }
    
    // Try to find IP pattern in the entire text
    let mut current_ip = String::new();
    let mut dot_count = 0;
    
    for c in text.chars() {
        if c.is_numeric() {
            current_ip.push(c);
        } else if c == '.' && !current_ip.is_empty() {
            current_ip.push(c);
            dot_count += 1;
        } else if !current_ip.is_empty() {
            if dot_count == 3 && validate_ip(&current_ip) {
                return Some(current_ip);
            }
            current_ip.clear();
            dot_count = 0;
        }
    }
    
    if dot_count == 3 && validate_ip(&current_ip) {
        return Some(current_ip);
    }
    
    None
}

fn extract_response_time(text: &str) -> Option<u32> {
    // Look for patterns like "(123 ms)" or "(123ms)" or "123 ms" or "123ms"
    let text_lower = text.to_lowercase();
    
    // Pattern 1: (xxx ms)
    if let Some(start) = text.find('(') {
        if let Some(ms_pos) = text_lower[start..].find("ms") {
            let num_part = &text[start + 1..start + ms_pos];
            let num_str: String = num_part.chars().filter(|c| c.is_numeric()).collect();
            if let Ok(num) = num_str.parse() {
                return Some(num);
            }
        }
    }
    
    // Pattern 2: xxx ms (without parentheses)
    if let Some(ms_pos) = text_lower.find("ms") {
        let before_ms = &text[..ms_pos];
        let parts: Vec<&str> = before_ms.split_whitespace().collect();
        if let Some(last) = parts.last() {
            let num_str: String = last.chars().filter(|c| c.is_numeric()).collect();
            if let Ok(num) = num_str.parse() {
                return Some(num);
            }
        }
    }
    
    None
}

fn extract_location(text: &str) -> String {
    // Look for location after last dash
    if let Some(dash_pos) = text.rfind('-') {
        let location = text[dash_pos + 1..].trim();
        let clean: String = location
            .chars()
            .take_while(|c| c.is_alphabetic() || c.is_whitespace() || *c == ',')
            .collect();
        if !clean.trim().is_empty() {
            return clean.trim().to_string();
        }
    }
    
    // Look for location in parentheses at the end
    if let Some(start) = text.rfind('(') {
        if let Some(end) = text[start..].find(')') {
            let location = &text[start + 1..start + end];
            if !location.contains("ms") && location.chars().any(|c| c.is_alphabetic()) {
                return location.trim().to_string();
            }
        }
    }
    
    "Unknown".to_string()
}

fn validate_ip(ip: &str) -> bool {
    if ip.is_empty() {
        return false;
    }
    
    let parts: Vec<&str> = ip.split('.').collect();

    if parts.len() != 4 {
        return false;
    }

    for part in &parts {
        if part.is_empty() {
            return false;
        }
        match part.parse::<u8>() {
            Ok(_) => continue,
            Err(_) => return false,
        }
    }
    
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_validation() {
        assert!(validate_ip("192.168.1.1"));
        assert!(validate_ip("8.8.8.8"));
        assert!(validate_ip("1.1.1.1"));
        assert!(validate_ip("255.255.255.255"));
        assert!(!validate_ip("256.1.1.1"));
        assert!(!validate_ip("192.168.1"));
        assert!(!validate_ip(""));
        assert!(!validate_ip("..."));
        assert!(!validate_ip("1.2.3"));
    }

    #[test]
    fn test_proxy_parsing() {
        let line = "PROXY LIVE ✅: 167.114.67.25 (989 ms) - Beauharnois";
        let proxy = parse_proxy_line(line).unwrap();
        assert_eq!(proxy.ip, "167.114.67.25");
        assert_eq!(proxy.response_time, 989);
    }

    #[test]
    fn test_proxy_parsing_alternate_format() {
        let line = "✅ 104.21.48.1 (150 ms) - Cloudflare";
        let proxy = parse_proxy_line(line).unwrap();
        assert_eq!(proxy.ip, "104.21.48.1");
    }

    #[test]
    fn test_ip_extraction() {
        assert_eq!(extract_ip_from_text("test 192.168.1.1 end"), Some("192.168.1.1".to_string()));
        assert_eq!(extract_ip_from_text("✅: 8.8.8.8 (100ms)"), Some("8.8.8.8".to_string()));
    }

    #[test]
    fn test_response_time() {
        assert_eq!(extract_response_time("test (123 ms) end"), Some(123));
        assert_eq!(extract_response_time("test (123ms) end"), Some(123));
        assert_eq!(extract_response_time("123 ms"), Some(123));
    }
}
