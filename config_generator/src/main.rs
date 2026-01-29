use std::fs;
use std::path::Path;
use std::env;

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

    // Print current working directory for debugging
    match env::current_dir() {
        Ok(path) => println!("📍 Current working directory: {}", path.display()),
        Err(e) => println!("⚠️ Could not get current directory: {}", e),
    }
    println!();

    let output_dir = "sub/configs";
    let scanner_output = "sub/ProxyIP-Daily.md";

    // Create directories with detailed error handling
    println!("📁 Creating output directories...");
    match fs::create_dir_all(output_dir) {
        Ok(_) => println!("   ✓ Created: {}", output_dir),
        Err(e) => {
            eprintln!("   ⚠️ Warning creating {}: {}", output_dir, e);
            // Try to continue anyway
        }
    }
    
    let qr_dir = format!("{}/qr_codes", output_dir);
    match fs::create_dir_all(&qr_dir) {
        Ok(_) => println!("   ✓ Created: {}", qr_dir),
        Err(e) => {
            eprintln!("   ⚠️ Warning creating {}: {}", qr_dir, e);
        }
    }
    println!();

    // Verify directories exist
    if !Path::new(output_dir).exists() {
        eprintln!("❌ Failed to create output directory: {}", output_dir);
        eprintln!("   Attempting to create with absolute path...");
        
        if let Ok(cwd) = env::current_dir() {
            let abs_path = cwd.join(output_dir);
            if let Err(e) = fs::create_dir_all(&abs_path) {
                eprintln!("❌ Still failed: {}", e);
            } else {
                println!("   ✓ Created with absolute path: {}", abs_path.display());
            }
        }
    }

    // Read proxies with detailed error handling
    println!("📄 Looking for scanner output: {}", scanner_output);
    let proxies = match read_proxy_list(scanner_output) {
        Ok(p) => {
            println!("   ✓ Successfully read {} proxies", p.len());
            p
        }
        Err(e) => {
            eprintln!("   ⚠️ Error reading proxy list: {}", e);
            eprintln!("   Continuing with empty proxy list...");
            Vec::new()
        }
    };
    println!();

    let config_generator = ConfigGenerator::new();
    let output_generator = OutputGenerator::new();
    let subscription_manager = SubscriptionManager::new(None);

    if proxies.is_empty() {
        println!("⚠️  No proxies found. Creating empty output files...\n");

        let empty_bundle = output_generator.create_empty_output();

        println!("📝 Saving empty files:");
        match output_generator.save_to_files(&empty_bundle, output_dir) {
            Ok(_) => println!("   ✓ Files saved successfully"),
            Err(e) => {
                eprintln!("   ❌ Error saving files: {}", e);
                // Try to create minimal files manually
                create_minimal_files(output_dir);
            }
        }
        
        match subscription_manager.save_qr_codes(&empty_bundle.all_configs, output_dir) {
            Ok(_) => println!("   ✓ QR codes directory prepared"),
            Err(e) => eprintln!("   ⚠️ Warning: Could not prepare QR codes: {}", e),
        }

        println!("\n✅ Empty output files created successfully!");
        println!("📝 Note: Files will be populated once live proxies are detected.\n");

        list_output_files(output_dir);
        return Ok(());
    }

    // Generate configs
    println!("⚙️  Generating configurations for {} proxies...", proxies.len());
    let mut all_configs = Vec::new();

    for (idx, proxy) in proxies.iter().enumerate() {
        let configs = config_generator.generate_configs(&proxy.ip, proxy.port);
        println!("   [{}/{}] Generated {} configs for {}", 
                 idx + 1, proxies.len(), configs.len(), proxy.ip);
        all_configs.extend(configs);
    }

    let total_generated = all_configs.len();
    println!("\n✅ Generated {} total configurations\n", total_generated);

    // Save generated configs BEFORE testing
    println!("📝 Saving all generated configs before testing...");
    let pre_test_bundle = output_generator.generate_output_without_testing(&all_configs);
    match output_generator.save_all_generated_configs(&pre_test_bundle, output_dir) {
        Ok(_) => println!("   ✓ Pre-test configs saved"),
        Err(e) => eprintln!("   ⚠️ Warning: Could not save pre-test configs: {}", e),
    }

    // Test configs
    println!("\n🔬 Testing configurations (this may take a while)...");
    let config_tester = ConfigTester::new(8, 30); // Reduced timeout and concurrency for GitHub Actions
    let test_results = config_tester.test_configs(all_configs).await;

    let working_count = test_results.iter().filter(|r| r.is_working).count();
    let failed_count = test_results.len() - working_count;
    println!(
        "\n✅ Testing complete: {}/{} configs are working ({} failed)\n",
        working_count,
        test_results.len(),
        failed_count
    );

    // Generate output with ALL configs (both working and failed)
    println!("📝 Generating output files:");
    let output_bundle = output_generator.generate_output(test_results);

    match output_generator.save_to_files(&output_bundle, output_dir) {
        Ok(_) => println!("   ✓ All output files saved"),
        Err(e) => {
            eprintln!("   ❌ Error saving output files: {}", e);
            create_minimal_files(output_dir);
        }
    }

    println!("\n📱 Generating QR codes:");
    match subscription_manager.save_qr_codes(&output_bundle.all_configs, output_dir) {
        Ok(_) => println!("   ✓ QR codes generated"),
        Err(e) => eprintln!("   ⚠️ Warning: Could not generate QR codes: {}", e),
    }

    println!("\n✨ Success! Output saved to: {}/", output_dir);
    println!("\n📊 Summary:");
    println!(
        "   Total Generated: {}",
        output_bundle.all_configs.len()
    );
    println!(
        "   Total Tested: {}",
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

fn create_minimal_files(output_dir: &str) {
    println!("   📝 Creating minimal fallback files...");
    
    let timestamp = chrono::Utc::now().to_rfc3339();
    
    // Create directory if needed
    let _ = fs::create_dir_all(output_dir);
    let _ = fs::create_dir_all(format!("{}/qr_codes", output_dir));
    
    // subscription.txt
    let _ = fs::write(
        format!("{}/subscription.txt", output_dir),
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, "")
    );
    
    // configs.json
    let json = format!(r#"{{
  "subscription_link": "",
  "working_subscription_link": "",
  "all_configs": [],
  "configs": [],
  "statistics": {{
    "total_configs": 0,
    "working_configs": 0,
    "failed_configs": 0,
    "success_rate": 0.0,
    "average_response_time_ms": 0.0,
    "fastest_response_time_ms": null,
    "slowest_response_time_ms": null
  }},
  "generated_at": "{}"
}}"#, timestamp);
    let _ = fs::write(format!("{}/configs.json", output_dir), json);
    
    // Other files
    let _ = fs::write(format!("{}/all_configs.txt", output_dir), "# No configurations generated");
    let _ = fs::write(format!("{}/config_links.txt", output_dir), "# No working configurations");
    let _ = fs::write(format!("{}/vless_configs.txt", output_dir), "# No VLESS configurations");
    let _ = fs::write(format!("{}/vmess_configs.txt", output_dir), "# No VMess configurations");
    let _ = fs::write(format!("{}/trojan_configs.txt", output_dir), "# No Trojan configurations");
    let _ = fs::write(format!("{}/shadowsocks_configs.txt", output_dir), "# No Shadowsocks configurations");
    let _ = fs::write(format!("{}/statistics.txt", output_dir), format!("Generated at: {}\nNo data available.", timestamp));
    let _ = fs::write(format!("{}/qr_codes/README.txt", output_dir), "No QR codes available");
    
    println!("   ✓ Minimal files created");
}

fn list_output_files(output_dir: &str) {
    println!("📂 Output files:");

    let files = vec![
        "subscription.txt",
        "working_subscription.txt",
        "configs.json",
        "all_configs.txt",
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
        match fs::read_dir(&qr_dir) {
            Ok(entries) => {
                let count = entries.count();
                println!("   ✓ qr_codes/ ({} files)", count);
            }
            Err(e) => println!("   ⚠️ qr_codes/ (error reading: {})", e),
        }
    } else {
        println!("   ✗ qr_codes/ (missing)");
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
    
    // Check if file exists
    let path = Path::new(file_path);
    if !path.exists() {
        // Try with absolute path
        if let Ok(cwd) = env::current_dir() {
            let abs_path = cwd.join(file_path);
            println!("   Trying absolute path: {}", abs_path.display());
            if !abs_path.exists() {
                println!("   ⚠️ File not found at either location");
                return Ok(Vec::new());
            }
            return read_proxy_file(&abs_path);
        }
        println!("   ⚠️ Scanner output file not found: {}", file_path);
        return Ok(Vec::new());
    }

    read_proxy_file(path)
}

fn read_proxy_file(path: &Path) -> Result<Vec<ProxyInfo>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    println!("   📄 File size: {} bytes", content.len());

    if content.trim().is_empty() {
        println!("   ⚠️ Scanner output file is empty");
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
            || trimmed.starts_with("##")
        {
            continue;
        }

        // Look for various proxy line formats
        if line.contains("PROXY LIVE") || line.contains("✅") {
            proxy_lines_found += 1;
            if let Some(proxy) = parse_proxy_line(line) {
                proxies.push(proxy);
            }
        } else if let Some(proxy) = try_parse_ip_line(line) {
            proxy_lines_found += 1;
            proxies.push(proxy);
        }
    }

    println!("   📊 Processed {} lines, found {} proxy lines, parsed {} proxies", 
             lines_processed, proxy_lines_found, proxies.len());
    
    if proxies.is_empty() && proxy_lines_found > 0 {
        println!("   ⚠️ Found proxy lines but couldn't parse them. Sample lines:");
        for (i, line) in content.lines().filter(|l| l.contains("PROXY") || l.contains("✅")).take(5).enumerate() {
            println!("      Line {}: {}", i + 1, line.chars().take(100).collect::<String>());
        }
    }

    Ok(proxies)
}

fn parse_proxy_line(line: &str) -> Option<ProxyInfo> {
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
    let parts: Vec<&str> = text.split_whitespace().collect();
    
    for part in &parts {
        let cleaned: String = part
            .chars()
            .filter(|c| c.is_numeric() || *c == '.')
            .collect();
        
        if validate_ip(&cleaned) {
            return Some(cleaned);
        }
        
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
    if ip.is_empty() || ip.len() < 7 || ip.len() > 15 {
        return false;
    }
    
    let parts: Vec<&str> = ip.split('.').collect();

    if parts.len() != 4 {
        return false;
    }

    for part in &parts {
        if part.is_empty() || part.len() > 3 {
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
    fn test_ip_extraction() {
        assert_eq!(extract_ip_from_text("test 192.168.1.1 end"), Some("192.168.1.1".to_string()));
        assert_eq!(extract_ip_from_text("✅: 8.8.8.8 (100ms)"), Some("8.8.8.8".to_string()));
    }
}
