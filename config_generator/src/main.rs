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

    // Ensure output directory exists
    fs::create_dir_all("sub/configs/qr_codes")?;

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
        let output_dir = "sub/configs";
        output_generator.save_to_files(&empty_bundle, output_dir)?;
        
        println!("\n✅ Empty output files created successfully!");
        println!("   Location: {}/", output_dir);
        println!("\n📝 Note: These files will be populated once live proxies are detected.");
        
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
    println!("🔍 Testing configurations (this may take a while)...");
    let config_tester = ConfigTester::new(10, 50); // 10 sec timeout, 50 concurrent
    let test_results = config_tester.test_configs(all_configs).await;
    
    let working_count = test_results.iter().filter(|r| r.is_working).count();
    println!("✅ Testing complete: {}/{} configs are working\n", 
             working_count, test_results.len());

    // Generate output
    println!("📁 Generating output files...");
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
    // Check if file exists
    if !std::path::Path::new(file_path).exists() {
        println!("⚠️  Scanner output file not found: {}", file_path);
        println!("   Creating empty proxy list...");
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(file_path)?;
    let mut proxies = Vec::new();

    // Debug: show first few lines to understand format
    let first_lines: Vec<&str> = content.lines().take(5).collect();
    println!("📄 Reading scanner output file (first 5 lines):");
    for (i, line) in first_lines.iter().enumerate() {
        println!("   Line {}: {}", i + 1, line);
    }

    for line in content.lines() {
        // Parse lines like: "PROXY LIVE ✅: 167.114.67.25 (989 ms) - Beauharnois"
        // Also handle variations without checkmark
        if line.contains("PROXY LIVE") && (line.contains("✅") || line.contains(":")) {
            if let Some(proxy) = parse_proxy_line(line) {
                proxies.push(proxy);
            }
        }
    }

    println!("\n✅ Successfully parsed {} proxies from file\n", proxies.len());

    Ok(proxies)
}

fn parse_proxy_line(line: &str) -> Option<ProxyInfo> {
    // Handle format: "PROXY LIVE ✅: 167.114.67.25 (989 ms) - Beauharnois"
    
    // First, try to find the IP address after the colon
    if let Some(colon_pos) = line.find(':') {
        let after_colon = &line[colon_pos + 1..];
        
        // Split by whitespace and find the IP
        let parts: Vec<&str> = after_colon.split_whitespace().collect();
        
        for part in parts.iter() {
            // Check if this looks like an IP address
            if part.contains('.') && !part.contains("✅") {
                // Clean up the IP (remove any trailing characters)
                let ip_candidate = part
                    .trim()
                    .trim_matches(|c: char| !c.is_numeric() && c != '.');
                
                // Validate IP format
                if validate_ip(ip_candidate) {
                    // Extract response time
                    let response_time = extract_number_before(line, "ms)").unwrap_or(0);
                    
                    // Extract location (everything after the last -)
                    let location = if let Some(dash_pos) = line.rfind('-') {
                        line[dash_pos + 1..].trim().to_string()
                    } else {
                        "Unknown".to_string()
                    };
                    
                    return Some(ProxyInfo {
                        ip: ip_candidate.to_string(),
                        port: 443, // Default port from scanner
                        location,
                        response_time,
                    });
                }
            }
        }
    }
    
    None
}

fn extract_number_before(text: &str, marker: &str) -> Option<u32> {
    if let Some(marker_pos) = text.find(marker) {
        let before_marker = &text[..marker_pos];
        
        // Find the last opening parenthesis before the marker
        if let Some(paren_pos) = before_marker.rfind('(') {
            let number_part = &before_marker[paren_pos + 1..];
            
            // Extract just the numbers
            let num_str: String = number_part
                .chars()
                .filter(|c| c.is_numeric())
                .collect();
            
            return num_str.parse::<u32>().ok();
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
        // Each part should be a valid number between 0-255
        if let Ok(num) = part.parse::<u8>() {
            true
        } else {
            false
        }
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
    
    #[test]
    fn test_proxy_parsing_variations() {
        // Test without checkmark
        let line1 = "PROXY LIVE: 155.138.128.135 (314 ms) - Toronto";
        let proxy1 = parse_proxy_line(line1);
        assert!(proxy1.is_some());
        assert_eq!(proxy1.unwrap().ip, "155.138.128.135");
        
        // Test with different spacing
        let line2 = "PROXY LIVE ✅:    167.114.67.25    (989 ms)  -  Beauharnois";
        let proxy2 = parse_proxy_line(line2);
        assert!(proxy2.is_some());
        assert_eq!(proxy2.unwrap().ip, "167.114.67.25");
    }
    
    #[test]
    fn test_number_extraction() {
        assert_eq!(extract_number_before("test (123 ms) end", "ms)"), Some(123));
        assert_eq!(extract_number_before("test (4567 ms) end", "ms)"), Some(4567));
        assert_eq!(extract_number_before("no number here ms)", "ms)"), None);
    }
}
