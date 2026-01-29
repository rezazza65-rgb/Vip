use std::fs;
use std::path::Path;

mod generator;
mod output;
mod tester;

use generator::ConfigGenerator;
use output::{OutputGenerator, SubscriptionManager};
use tester::ConfigTester;

const SCANNER_FILE: &str = "sub/ProxyIP-Daily.md";
const OUTPUT_DIR: &str = "sub/configs";
const DEFAULT_PORT: u16 = 443;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Proxy Configuration Generator");
    println!("=================================\n");

    // Create output directories
    fs::create_dir_all(OUTPUT_DIR)?;
    fs::create_dir_all(format!("{}/qr_codes", OUTPUT_DIR))?;

    // Step 1: Read LIVE IPs from scanner
    println!("📄 Reading scanner output: {}", SCANNER_FILE);
    let live_ips = extract_live_ips_from_scanner(SCANNER_FILE);
    
    if live_ips.is_empty() {
        println!("\n❌ No PROXY LIVE IPs found in scanner output!");
        println!("   Make sure scanner has run and created the file.");
        
        // Create empty files
        let output_gen = OutputGenerator::new();
        output_gen.save_empty_files(OUTPUT_DIR)?;
        return Ok(());
    }

    println!("\n✅ Found {} PROXY LIVE IPs:\n", live_ips.len());
    for (i, ip) in live_ips.iter().enumerate() {
        println!("   {}. {}", i + 1, ip);
    }

    // Step 2: Generate configs for each IP
    println!("\n⚙️ Generating configurations...");
    let config_gen = ConfigGenerator::new();
    let mut all_configs = Vec::new();

    for ip in &live_ips {
        let configs = config_gen.generate_all_configs(ip, DEFAULT_PORT);
        println!("   ✓ {} → {} configs", ip, configs.len());
        all_configs.extend(configs);
    }

    println!("\n📊 Total: {} configs generated", all_configs.len());

    // Step 3: Save all configs immediately
    println!("\n📝 Saving configurations...");
    let output_gen = OutputGenerator::new();
    let bundle = output_gen.create_bundle(&all_configs);
    output_gen.save_all_files(&bundle, OUTPUT_DIR)?;

    // Step 4: Generate QR codes
    println!("\n📱 Generating QR codes...");
    let sub_manager = SubscriptionManager::new();
    sub_manager.generate_qr_codes(&bundle.all_configs, OUTPUT_DIR)?;

    // Step 5: Test configs (optional)
    println!("\n🔬 Testing configurations...");
    let tester = ConfigTester::new(5, 20);
    let results = tester.test_all(all_configs).await;
    
    let working = results.iter().filter(|r| r.is_working).count();
    println!("   ✓ {}/{} working", working, results.len());

    // Step 6: Save final results with test data
    let final_bundle = output_gen.create_bundle_with_results(&results);
    output_gen.save_all_files(&final_bundle, OUTPUT_DIR)?;

    // Summary
    println!("\n" + &"=".repeat(50));
    println!("✨ COMPLETE!");
    println!("{}", "=".repeat(50));
    println!("\n📊 Summary:");
    println!("   Live IPs from scanner: {}", live_ips.len());
    println!("   Total configs: {}", final_bundle.all_configs.len());
    println!("   Working configs: {}", working);
    println!("\n📂 Output: {}/", OUTPUT_DIR);
    
    print_file_summary(OUTPUT_DIR);

    Ok(())
}

/// Extract ONLY IPs from lines containing "PROXY LIVE"
fn extract_live_ips_from_scanner(file_path: &str) -> Vec<String> {
    let path = Path::new(file_path);
    
    if !path.exists() {
        println!("   ❌ File not found: {}", file_path);
        return Vec::new();
    }

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            println!("   ❌ Cannot read file: {}", e);
            return Vec::new();
        }
    };

    println!("   📊 File size: {} bytes", content.len());

    let mut live_ips = Vec::new();
    let mut live_lines = 0;
    let mut dead_lines = 0;

    for line in content.lines() {
        let upper = line.to_uppercase();
        
        // Skip DEAD proxies
        if upper.contains("DEAD") || line.contains("❌") {
            dead_lines += 1;
            continue;
        }
        
        // Only process PROXY LIVE lines
        if upper.contains("PROXY LIVE") || (upper.contains("LIVE") && line.contains("✅")) || line.contains("🟩") {
            live_lines += 1;
            
            // Extract IP from this line
            if let Some(ip) = find_ip_in_line(line) {
                // Avoid duplicates
                if !live_ips.contains(&ip) {
                    live_ips.push(ip);
                }
            }
        }
    }

    println!("   📊 Found: {} LIVE lines, {} DEAD lines", live_lines, dead_lines);
    println!("   📊 Extracted: {} unique IPs", live_ips.len());

    live_ips
}

/// Find IP address in a line
fn find_ip_in_line(line: &str) -> Option<String> {
    // Split by common delimiters and find IP pattern
    for part in line.split(|c: char| c.is_whitespace() || c == ':' || c == '(' || c == ')') {
        let cleaned: String = part.chars()
            .filter(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        
        if is_valid_public_ip(&cleaned) {
            return Some(cleaned);
        }
    }
    
    // Fallback: scan character by character
    let mut current = String::new();
    let mut dots = 0;
    
    for ch in line.chars() {
        if ch.is_ascii_digit() {
            current.push(ch);
        } else if ch == '.' && !current.is_empty() && dots < 3 {
            current.push(ch);
            dots += 1;
        } else {
            if dots == 3 && is_valid_public_ip(&current) {
                return Some(current);
            }
            current.clear();
            dots = 0;
        }
    }
    
    if dots == 3 && is_valid_public_ip(&current) {
        return Some(current);
    }
    
    None
}

/// Validate IP and ensure it's public (not private/localhost)
fn is_valid_public_ip(ip: &str) -> bool {
    let parts: Vec<&str> = ip.split('.').collect();
    
    if parts.len() != 4 {
        return false;
    }
    
    let mut octets = Vec::new();
    for part in &parts {
        if part.is_empty() || part.len() > 3 {
            return false;
        }
        match part.parse::<u8>() {
            Ok(n) => octets.push(n),
            Err(_) => return false,
        }
    }
    
    // Exclude private and special IPs
    let first = octets[0];
    let second = octets[1];
    
    // 0.x.x.x - Invalid
    if first == 0 { return false; }
    // 10.x.x.x - Private
    if first == 10 { return false; }
    // 127.x.x.x - Localhost
    if first == 127 { return false; }
    // 172.16-31.x.x - Private
    if first == 172 && (16..=31).contains(&second) { return false; }
    // 192.168.x.x - Private
    if first == 192 && second == 168 { return false; }
    // 169.254.x.x - Link-local
    if first == 169 && second == 254 { return false; }
    
    true
}

fn print_file_summary(dir: &str) {
    println!("\n📂 Generated files:");
    
    let files = [
        ("all_configs.txt", "All generated configs"),
        ("config_links.txt", "Working configs only"),
        ("subscription.txt", "Base64 subscription"),
        ("vless_configs.txt", "VLESS configs"),
        ("vmess_configs.txt", "VMess configs"),
        ("trojan_configs.txt", "Trojan configs"),
        ("statistics.txt", "Test statistics"),
    ];
    
    for (file, desc) in files {
        let path = Path::new(dir).join(file);
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                let count = content.lines()
                    .filter(|l| !l.starts_with('#') && !l.is_empty())
                    .count();
                println!("   ✓ {} - {} ({} lines)", file, desc, count);
            }
        } else {
            println!("   ✗ {} - Missing", file);
        }
    }
    
    let qr_path = Path::new(dir).join("qr_codes");
    if qr_path.exists() {
        if let Ok(entries) = fs::read_dir(&qr_path) {
            let count = entries.filter(|e| e.is_ok()).count();
            println!("   ✓ qr_codes/ - {} files", count);
        }
    }
}
