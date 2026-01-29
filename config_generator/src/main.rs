use std::fs;
use std::path::Path;
use std::env;

// استفاده از کتابخانه config_generator
use config_generator::{
    ConfigGenerator, OutputGenerator, SubscriptionManager, 
    ConfigTester, ProxyInfo
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Advanced Proxy Configuration Generator");
    println!("==========================================\n");

    match env::current_dir() {
        Ok(path) => println!("📍 Current working directory: {}", path.display()),
        Err(e) => println!("⚠️ Could not get current directory: {}", e),
    }
    println!();

    let output_dir = "../sub/configs";
    let scanner_output = "../sub/ProxyIP-Daily.md";

    println!("📁 Creating output directories...");
    fs::create_dir_all(output_dir).ok();
    fs::create_dir_all(format!("{}/qr_codes", output_dir)).ok();
    println!("   ✓ Directories ready\n");

    // Read ONLY LIVE proxies (port 443)
    let proxies = read_live_proxy_list(scanner_output);
    println!("📡 Found {} LIVE proxies (port 443) from scanner\n", proxies.len());

    let config_generator = ConfigGenerator::new();
    let output_generator = OutputGenerator::new();
    let subscription_manager = SubscriptionManager::new();

    let mut all_configs = Vec::new();

    if proxies.is_empty() {
        println!("⚠️  No LIVE proxies found from scanner.");
        println!("❌ Cannot generate configs without valid proxy IPs\n");
        println!("💡 Please run the proxy scanner first to get live IPs");
        return Ok(());
    }

    println!("⚙️  Generating configurations for {} LIVE proxies...", proxies.len());
    
    for (idx, proxy) in proxies.iter().enumerate() {
        let configs = config_generator.generate_configs(&proxy.ip, proxy.port);
        println!("   [{}/{}] Generated {} configs for {} ({}ms) - {}", 
                 idx + 1, proxies.len(), configs.len(), proxy.ip, 
                 proxy.response_time, proxy.location);
        all_configs.extend(configs);
    }

    let total_generated = all_configs.len();
    println!("\n✅ Generated {} total configurations from {} LIVE IPs\n", 
             total_generated, proxies.len());

    // Save configs BEFORE testing
    println!("📝 Saving all generated configs (before testing)...");
    let pre_test_bundle = output_generator.create_bundle_from_configs(&all_configs);
    output_generator.save_to_files(&pre_test_bundle, output_dir)?;
    subscription_manager.save_qr_codes(&pre_test_bundle.all_configs, output_dir)?;
    println!("   ✓ All configs saved\n");

    // Test configs
    println!("🔬 Testing configurations...");
    let config_tester = ConfigTester::new(5, 20);
    let test_results = config_tester.test_configs(all_configs.clone()).await;

    let working_count = test_results.iter().filter(|r| r.is_working).count();
    println!(
        "\n✅ Testing complete: {}/{} working\n",
        working_count,
        test_results.len()
    );

    // Save final results with test data
    println!("📝 Saving final results with test data...");
    let final_bundle = output_generator.create_bundle_from_results(test_results);
    output_generator.save_to_files(&final_bundle, output_dir)?;
    subscription_manager.save_qr_codes(&final_bundle.all_configs, output_dir)?;

    println!("\n✨ Success! Output saved to: {}/", output_dir);
    println!("\n📊 Summary:");
    println!("   Source LIVE IPs: {}", proxies.len());
    println!("   Total Generated: {}", final_bundle.all_configs.len());
    println!("   Working: {}", final_bundle.statistics.working_configs);
    println!("   Failed: {}", final_bundle.statistics.failed_configs);
    println!("   Success Rate: {:.2}%", final_bundle.statistics.success_rate);

    println!();
    list_output_files(output_dir);

    Ok(())
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
            if let Ok(content) = fs::read_to_string(&path) {
                let lines = content.lines().count();
                println!("   ✓ {} ({} lines)", file, lines);
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

fn read_live_proxy_list(file_path: &str) -> Vec<ProxyInfo> {
    println!("📄 Reading scanner output: {}", file_path);
    
    let path = Path::new(file_path);
    if !path.exists() {
        println!("   ⚠️ File not found");
        return Vec::new();
    }

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            println!("   ⚠️ Error reading file: {}", e);
            return Vec::new();
        }
    };

    println!("   📊 File size: {} bytes", content.len());
    
    if content.is_empty() {
        println!("   ⚠️ File is empty");
        return Vec::new();
    }

    let mut proxies = Vec::new();
    let mut live_count = 0;
    let mut dead_count = 0;

    for line in content.lines() {
        let trimmed = line.trim();
        
        // Skip empty lines and headers
        if trimmed.is_empty() || 
           trimmed.starts_with('#') || 
           trimmed.starts_with('|') || 
           trimmed.starts_with('-') ||
           trimmed.starts_with('[') ||
           trimmed.starts_with('<') ||
           trimmed.starts_with('>') ||
           trimmed.starts_with("```") ||
           trimmed.starts_with("##") {
            continue;
        }

        // ✅ ONLY process LIVE proxies - SKIP DEAD ones
        if line.contains("PROXY LIVE") && line.contains("🟩") {
            live_count += 1;
            
            if let Some(ip) = extract_ip(line) {
                let response_time = extract_time(line);
                let location = extract_location(line);
                
                let proxy = ProxyInfo {
                    ip: ip.clone(),
                    port: 443,  // ✅ ALWAYS port 443
                    location,
                    response_time,
                };
                
                println!("   ✅ LIVE: {} ({}ms) - {}", ip, proxy.response_time, proxy.location);
                proxies.push(proxy);
            }
        } 
        // ❌ Skip DEAD proxies
        else if line.contains("PROXY DEAD") && line.contains("❌") {
            dead_count += 1;
            if let Some(ip) = extract_ip(line) {
                println!("   ❌ DEAD: {} (skipped)", ip);
            }
        }
    }

    println!("\n   📊 Statistics:");
    println!("      LIVE proxies: {} ✅", live_count);
    println!("      DEAD proxies: {} ❌ (skipped)", dead_count);
    println!("      Valid IPs extracted: {}", proxies.len());

    proxies
}

fn extract_ip(text: &str) -> Option<String> {
    let mut result = String::new();
    let mut dot_count = 0;
    let mut num_start = false;
    
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            result.push(ch);
            num_start = true;
        } else if ch == '.' && num_start && dot_count < 3 {
            result.push(ch);
            dot_count += 1;
        } else if num_start && dot_count == 3 {
            if is_valid_ip(&result) {
                return Some(result);
            }
            result.clear();
            dot_count = 0;
            num_start = false;
        } else {
            if num_start && dot_count == 3 {
                if is_valid_ip(&result) {
                    return Some(result);
                }
            }
            result.clear();
            dot_count = 0;
            num_start = false;
        }
    }
    
    if dot_count == 3 && is_valid_ip(&result) {
        return Some(result);
    }
    
    None
}

fn is_valid_ip(ip: &str) -> bool {
    let parts: Vec<&str> = ip.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    
    for part in parts {
        if part.is_empty() || part.len() > 3 {
            return false;
        }
        match part.parse::<u16>() {
            Ok(n) if n <= 255 => continue,
            _ => return false,
        }
    }
    
    true
}

fn extract_time(text: &str) -> u32 {
    if let Some(start) = text.find('(') {
        if let Some(end) = text.find("ms") {
            if end > start {
                let num_str: String = text[start+1..end]
                    .chars()
                    .filter(|c| c.is_ascii_digit())
                    .collect();
                return num_str.parse().unwrap_or(0);
            }
        }
    }
    0
}

fn extract_location(text: &str) -> String {
    if let Some(pos) = text.rfind('-') {
        let loc = text[pos+1..].trim();
        let clean: String = loc.chars()
            .take_while(|c| c.is_alphabetic() || c.is_whitespace())
            .collect();
        if !clean.is_empty() {
            return clean.trim().to_string();
        }
    }
    "Unknown".to_string()
}
