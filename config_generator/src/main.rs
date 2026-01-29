use std::fs;
use std::path::Path;

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

    // Ensure output directory exists
    let output_dir = "sub/configs";
    fs::create_dir_all(format!("{}/qr_codes", output_dir))?;
    println!("📁 Output directory: {}", output_dir);

    // Read proxy IPs from the scanner output
    let proxies = read_proxy_list("sub/ProxyIP-Daily.md")?;
    println!("📡 Found {} live proxies from scanner\n", proxies.len());

    // Initialize components
    let config_generator = ConfigGenerator::new();
    let output_generator = OutputGenerator::new();
    let subscription_manager = SubscriptionManager::new(None);

    if proxies.is_empty() {
        println!("⚠️  No proxies found. Creating empty output files...");
        
        // Create empty output bundle
        let empty_bundle = output_generator.create_empty_output();
        
        // Save empty files to disk
        match output_generator.save_to_files(&empty_bundle, output_dir) {
            Ok(()) => println!("✓ Empty config files created"),
            Err(e) => eprintln!("❌ Failed to save config files: {}", e),
        }
        
        // Generate empty QR codes directory
        match subscription_manager.save_qr_codes(&empty_bundle.configs, output_dir) {
            Ok(()) => println!("✓ QR code directory initialized"),
            Err(e) => eprintln!("❌ Failed to initialize QR codes: {}", e),
        }
        
        println!("\n✅ Empty output files created successfully!");
        println!("   Location: {}/", output_dir);
        println!("\n📝 Note: These files will be populated once live proxies are detected.");
        
        // List created files
        list_output_files(output_dir);
        
        return Ok(());
    }

    // Generate configurations for all proxies
    println!("⚙️  Generating configurations...");
    let mut all_configs = Vec::new();
    
    for proxy in &proxies {
        let configs = config_generator.generate_configs(&proxy.ip, proxy.port);
        all_configs.extend(configs);
    }

    println!("✅ Generated {} total configurations\n", all_configs.len());

    // Test all configurations
    println!("🔬 Testing configurations (this may take a while)...");
    let config_tester = ConfigTester::new(10, 50); // 10 sec timeout, 50 concurrent
    let test_results = config_tester.test_configs(all_configs).await;
    
    let working_count = test_results.iter().filter(|r| r.is_working).count();
    println!("✅ Testing complete: {}/{} configs are working\n", 
             working_count, test_results.len());

    // Generate output
    println!("📝 Generating output files...");
    let output_bundle = output_generator.generate_output(test_results);
    
    // Save to files
    match output_generator.save_to_files(&output_bundle, output_dir) {
        Ok(()) => println!("✓ Config files saved"),
        Err(e) => {
            eprintln!("❌ Failed to save config files: {}", e);
            return Err(e.into());
        }
    }
    
    // Generate QR codes
    println!("\n📱 Generating QR codes...");
    match subscription_manager.save_qr_codes(&output_bundle.configs, output_dir) {
        Ok(()) => println!("✓ QR codes generated"),
        Err(e) => eprintln!("⚠️ QR code generation had issues: {}", e),
    }

    println!("\n✨ Success! Output saved to: {}/", output_dir);
    println!("\n📊 Summary:");
    println!("   Total Configs: {}", output_bundle.statistics.total_configs);
    println!("   Working: {}", output_bundle.statistics.working_configs);
    println!("   Failed: {}", output_bundle.statistics.failed_configs);
    println!("   Success Rate: {:.2}%", output_bundle.statistics.success_rate);
    println!("   Avg Response: {:.2} ms", output_bundle.statistics.average_response_time_ms);

    // List created files
    list_output_files(output_dir);

    Ok(())
}

fn list_output_files(output_dir: &str) {
    println!("\n📂 Created files:");
    
    let expected_files = vec![
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

    for file in expected_files {
        let path = Path::new(output_dir).join(file);
        if path.exists() {
            if let Ok(metadata) = fs::metadata(&path) {
                println!("   ✓ {} ({} bytes)", file, metadata.len());
            } else {
                println!("   ✓ {}", file);
            }
        } else {
            println!("   ✗ {} (missing)", file);
        }
    }

    // Check QR codes directory
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
    // Check if file exists
    if !Path::new(file_path).exists() {
        println!("⚠️  Scanner output file not found: {}", file_path);
        println!("   Creating empty proxy list...");
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(file_path)?;
    
    // If file is empty or too small
    if content.trim().is_empty() || content.len() < 50 {
        println!("⚠️  Scanner output file is empty or too small");
        return Ok(Vec::new());
    }
    
    let mut proxies = Vec::new();

    // Debug: show first few lines to understand format
    let first_lines: Vec<&str> = content.lines().take(10).collect();
    println!("📄 Reading scanner output file (first 10 lines):");
    for (i, line) in first_lines.iter().enumerate() {
        let truncated = if line.len() > 80 {
            format!("{}...", &line[..80])
        } else {
            line.to_string()
        };
        println!("   Line {}: {}", i + 1, truncated);
    }
    println!();

    // Parse each line looking for proxy information
    let mut parse_attempts = 0;
    let mut parse_failures = 0;

    for line in content.lines() {
        // Skip HTML tags and markdown formatting
        let trimmed = line.trim();
        if trimmed.starts_with('<') || 
           trimmed.starts_with('>') || 
           trimmed.starts_with('#') ||
           trimmed.starts_with('|') ||
           trimmed.starts_with('[') ||
           trimmed.starts_with('*') ||
           trimmed.starts_with('-') ||
           trimmed.is_empty() {
            continue;
        }
        
        // Look for lines with "PROXY LIVE" indicator
        if line.contains("PROXY LIVE") {
            parse_attempts += 1;
            if let Some(proxy) = parse_proxy_line(line) {
                proxies.push(proxy);
            } else {
                parse_failures += 1;
                if parse_failures <= 5 {
                    println!("   ⚠️  Parse failed: {}", 
                        if line.len() > 60 { format!("{}...", &line[..60]) } else { line.to_string() });
                }
            }
        }
    }

    if parse_failures > 5 {
        println!("   ... and {} more parse failures", parse_failures - 5);
    }

    println!("\n✅ Parsed {}/{} proxy entries successfully\n", 
             proxies.len(), parse_attempts);

    Ok(proxies)
}

fn parse_proxy_line(line: &str) -> Option<ProxyInfo> {
    // Handle multiple formats:
    // Format 1: "PROXY LIVE ✅: 167.114.67.25 (989 ms) - Beauharnois"
    // Format 2: "PROXY LIVE: 155.138.128.135 (314 ms) - Toronto"
    // Format 3: "1043 PROXY LIVE ✅: 94.131.101.246 (1077 ms) - Secaucus"
    
    // Find the colon after "PROXY LIVE"
    let colon_pos = line.find(':')?;
    let after_colon = &line[colon_pos + 1..];
    
    // Split by whitespace to find components
    let parts: Vec<&str> = after_colon.split_whitespace().collect();
    
    for part in &parts {
        // Skip if it contains special characters that aren't part of IP
        if part.contains('✅') || part.contains("ms") || part.contains('(') || part.contains(')') {
            continue;
        }
        
        // Check if this looks like an IP address
        let ip_candidate = part.trim().trim_matches(|c: char| !c.is_numeric() && c != '.');
        
        // Validate IP format
        if validate_ip(ip_candidate) {
            // Extract response time - look for pattern like "(989 ms)"
            let response_time = extract_response_time(line).unwrap_or(0);
            
            // Extract location (everything after the last hyphen, excluding parentheses content)
            let location = extract_location(line);
            
            return Some(ProxyInfo {
                ip: ip_candidate.to_string(),
                port: 443, // Default port
                location,
                response_time,
            });
        }
    }
    
    None
}

fn extract_response_time(text: &str) -> Option<u32> {
    // Find pattern like "(989 ms)"
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
    // Find the last dash and extract location
    if let Some(dash_pos) = text.rfind('-') {
        let location = text[dash_pos + 1..].trim();
        // Clean up any trailing content
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
        assert!(validate_ip("255.255.255.255"));
        assert!(validate_ip("0.0.0.0"));
        assert!(!validate_ip("256.1.1.1"));
        assert!(!validate_ip("192.168.1"));
        assert!(!validate_ip("invalid"));
        assert!(!validate_ip("192.168.1.1.1"));
    }

    #[test]
    fn test_proxy_parsing_basic() {
        let line = "PROXY LIVE ✅: 167.114.67.25 (989 ms) - Beauharnois";
        let proxy = parse_proxy_line(line);
        
        assert!(proxy.is_some());
        let proxy = proxy.unwrap();
        assert_eq!(proxy.ip, "167.114.67.25");
        assert_eq!(proxy.port, 443);
        assert_eq!(proxy.response_time, 989);
        assert_eq!(proxy.location, "Beauharnois");
    }
    
    #[test]
    fn test_proxy_parsing_with_number_prefix() {
        let line = "1043 PROXY LIVE ✅: 94.131.101.246 (1077 ms) - Secaucus";
        let proxy = parse_proxy_line(line);
        
        assert!(proxy.is_some());
        let proxy = proxy.unwrap();
        assert_eq!(proxy.ip, "94.131.101.246");
        assert_eq!(proxy.response_time, 1077);
        assert_eq!(proxy.location, "Secaucus");
    }
    
    #[test]
    fn test_proxy_parsing_without_checkmark() {
        let line = "PROXY LIVE: 155.138.128.135 (314 ms) - Toronto";
        let proxy = parse_proxy_line(line);
        
        assert!(proxy.is_some());
        let p = proxy.unwrap();
        assert_eq!(p.ip, "155.138.128.135");
        assert_eq!(p.response_time, 314);
    }
    
    #[test]
    fn test_response_time_extraction() {
        assert_eq!(extract_response_time("test (123 ms) end"), Some(123));
        assert_eq!(extract_response_time("test (4567 ms) end"), Some(4567));
        assert_eq!(extract_response_time("no parens here"), None);
    }

    #[test]
    fn test_location_extraction() {
        assert_eq!(extract_location("something - Toronto"), "Toronto");
        assert_eq!(extract_location("something - New York"), "New York");
        assert_eq!(extract_location("no dash here"), "Unknown");
    }
}
