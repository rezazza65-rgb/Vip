use base64::{Engine as _, engine::general_purpose};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub protocol: String,
    pub transmission: String,
    pub address: String,
    pub port: u16,
    pub uuid: String,
    pub sni: String,
    pub host: String,
    pub path: String,
    pub alpn: String,
    pub fingerprint: String,
    pub service_name: String,
}

pub struct ConfigGenerator {
    snis: Vec<&'static str>,
    paths: Vec<&'static str>,
    fingerprints: Vec<&'static str>,
}

impl Default for ConfigGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigGenerator {
    pub fn new() -> Self {
        Self {
            snis: vec![
                "www.speedtest.net",
                "www.google.com",
                "www.cloudflare.com",
                "discord.com",
                "www.microsoft.com",
                "www.apple.com",
                "www.amazon.com",
            ],
            paths: vec!["/", "/ws", "/vless", "/vmess", "/trojan"],
            fingerprints: vec!["chrome", "firefox", "safari", "edge", "random"],
        }
    }

    /// Generate all config types for a single IP
    pub fn generate_all_configs(&self, ip: &str, port: u16) -> Vec<ProxyConfig> {
        let mut rng = rand::thread_rng();
        let mut configs = Vec::new();

        // VLESS + WebSocket + TLS
        configs.push(self.vless_ws(ip, port, &mut rng));
        
        // VLESS + TCP + TLS  
        configs.push(self.vless_tcp(ip, port, &mut rng));
        
        // VLESS + gRPC + TLS
        configs.push(self.vless_grpc(ip, port, &mut rng));
        
        // VMess + WebSocket + TLS
        configs.push(self.vmess_ws(ip, port, &mut rng));
        
        // Trojan + WebSocket + TLS
        configs.push(self.trojan_ws(ip, port, &mut rng));
        
        // Trojan + gRPC + TLS
        configs.push(self.trojan_grpc(ip, port, &mut rng));

        configs
    }

    fn new_uuid() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    fn vless_ws(&self, ip: &str, port: u16, rng: &mut impl Rng) -> ProxyConfig {
        let sni = self.snis[rng.gen_range(0..self.snis.len())];
        ProxyConfig {
            protocol: "vless".into(),
            transmission: "ws".into(),
            address: ip.into(),
            port,
            uuid: Self::new_uuid(),
            sni: sni.into(),
            host: sni.into(),
            path: self.paths[rng.gen_range(0..self.paths.len())].into(),
            alpn: "h2,http/1.1".into(),
            fingerprint: self.fingerprints[rng.gen_range(0..self.fingerprints.len())].into(),
            service_name: String::new(),
        }
    }

    fn vless_tcp(&self, ip: &str, port: u16, rng: &mut impl Rng) -> ProxyConfig {
        let sni = self.snis[rng.gen_range(0..self.snis.len())];
        ProxyConfig {
            protocol: "vless".into(),
            transmission: "tcp".into(),
            address: ip.into(),
            port,
            uuid: Self::new_uuid(),
            sni: sni.into(),
            host: sni.into(),
            path: String::new(),
            alpn: "h2,http/1.1".into(),
            fingerprint: self.fingerprints[rng.gen_range(0..self.fingerprints.len())].into(),
            service_name: String::new(),
        }
    }

    fn vless_grpc(&self, ip: &str, port: u16, rng: &mut impl Rng) -> ProxyConfig {
        let sni = self.snis[rng.gen_range(0..self.snis.len())];
        ProxyConfig {
            protocol: "vless".into(),
            transmission: "grpc".into(),
            address: ip.into(),
            port,
            uuid: Self::new_uuid(),
            sni: sni.into(),
            host: String::new(),
            path: String::new(),
            alpn: "h2".into(),
            fingerprint: self.fingerprints[rng.gen_range(0..self.fingerprints.len())].into(),
            service_name: format!("vl{}", rng.gen_range(1000..9999)),
        }
    }

    fn vmess_ws(&self, ip: &str, port: u16, rng: &mut impl Rng) -> ProxyConfig {
        let sni = self.snis[rng.gen_range(0..self.snis.len())];
        ProxyConfig {
            protocol: "vmess".into(),
            transmission: "ws".into(),
            address: ip.into(),
            port,
            uuid: Self::new_uuid(),
            sni: sni.into(),
            host: sni.into(),
            path: self.paths[rng.gen_range(0..self.paths.len())].into(),
            alpn: "h2,http/1.1".into(),
            fingerprint: self.fingerprints[rng.gen_range(0..self.fingerprints.len())].into(),
            service_name: String::new(),
        }
    }

    fn trojan_ws(&self, ip: &str, port: u16, rng: &mut impl Rng) -> ProxyConfig {
        let sni = self.snis[rng.gen_range(0..self.snis.len())];
        ProxyConfig {
            protocol: "trojan".into(),
            transmission: "ws".into(),
            address: ip.into(),
            port,
            uuid: Self::new_uuid(),
            sni: sni.into(),
            host: sni.into(),
            path: self.paths[rng.gen_range(0..self.paths.len())].into(),
            alpn: "h2,http/1.1".into(),
            fingerprint: self.fingerprints[rng.gen_range(0..self.fingerprints.len())].into(),
            service_name: String::new(),
        }
    }

    fn trojan_grpc(&self, ip: &str, port: u16, rng: &mut impl Rng) -> ProxyConfig {
        let sni = self.snis[rng.gen_range(0..self.snis.len())];
        ProxyConfig {
            protocol: "trojan".into(),
            transmission: "grpc".into(),
            address: ip.into(),
            port,
            uuid: Self::new_uuid(),
            sni: sni.into(),
            host: String::new(),
            path: String::new(),
            alpn: "h2".into(),
            fingerprint: self.fingerprints[rng.gen_range(0..self.fingerprints.len())].into(),
            service_name: format!("tr{}", rng.gen_range(1000..9999)),
        }
    }

    /// Convert config to subscription link
    pub fn to_link(&self, c: &ProxyConfig) -> String {
        match c.protocol.as_str() {
            "vless" => self.vless_link(c),
            "vmess" => self.vmess_link(c),
            "trojan" => self.trojan_link(c),
            _ => String::new(),
        }
    }

    fn vless_link(&self, c: &ProxyConfig) -> String {
        let mut params = vec![
            format!("type={}", c.transmission),
            "security=tls".into(),
            "encryption=none".into(),
        ];

        if !c.sni.is_empty() {
            params.push(format!("sni={}", c.sni));
        }
        if !c.host.is_empty() {
            params.push(format!("host={}", c.host));
        }
        if !c.path.is_empty() {
            params.push(format!("path={}", urlencoding::encode(&c.path)));
        }
        if !c.alpn.is_empty() {
            params.push(format!("alpn={}", urlencoding::encode(&c.alpn)));
        }
        if !c.fingerprint.is_empty() {
            params.push(format!("fp={}", c.fingerprint));
        }
        if !c.service_name.is_empty() {
            params.push(format!("serviceName={}", c.service_name));
        }

        let name = format!("VLESS-{}-{}", c.transmission.to_uppercase(), c.address);
        
        format!(
            "vless://{}@{}:{}?{}#{}",
            c.uuid, c.address, c.port,
            params.join("&"),
            urlencoding::encode(&name)
        )
    }

    fn vmess_link(&self, c: &ProxyConfig) -> String {
        let json = serde_json::json!({
            "v": "2",
            "ps": format!("VMess-{}-{}", c.transmission.to_uppercase(), c.address),
            "add": c.address,
            "port": c.port.to_string(),
            "id": c.uuid,
            "aid": "0",
            "scy": "auto",
            "net": c.transmission,
            "type": "none",
            "host": c.host,
            "path": c.path,
            "tls": "tls",
            "sni": c.sni,
            "alpn": c.alpn,
            "fp": c.fingerprint
        });
        
        format!("vmess://{}", general_purpose::STANDARD.encode(json.to_string()))
    }

    fn trojan_link(&self, c: &ProxyConfig) -> String {
        let mut params = vec![
            format!("type={}", c.transmission),
            "security=tls".into(),
        ];

        if !c.sni.is_empty() {
            params.push(format!("sni={}", c.sni));
        }
        if !c.host.is_empty() {
            params.push(format!("host={}", c.host));
        }
        if !c.path.is_empty() {
            params.push(format!("path={}", urlencoding::encode(&c.path)));
        }
        if !c.alpn.is_empty() {
            params.push(format!("alpn={}", urlencoding::encode(&c.alpn)));
        }
        if !c.fingerprint.is_empty() {
            params.push(format!("fp={}", c.fingerprint));
        }
        if !c.service_name.is_empty() {
            params.push(format!("serviceName={}", c.service_name));
        }

        let name = format!("Trojan-{}-{}", c.transmission.to_uppercase(), c.address);
        
        format!(
            "trojan://{}@{}:{}?{}#{}",
            c.uuid, c.address, c.port,
            params.join("&"),
            urlencoding::encode(&name)
        )
    }
}
