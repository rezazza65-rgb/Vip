use base64::{Engine as _, engine::general_purpose};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub protocol: Protocol,
    pub transmission: Transmission,
    pub address: String,
    pub port: u16,
    pub id: String,
    pub security: Security,
    pub sni: String,
    pub alpn: Vec<String>,
    pub fingerprint: String,
    pub path: Option<String>,
    pub host: Option<String>,
    pub service_name: Option<String>,
    pub public_key: Option<String>,
    pub short_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Protocol {
    VLESS,
    VMess,
    Trojan,
    Shadowsocks,
}

impl Protocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Protocol::VLESS => "VLESS",
            Protocol::VMess => "VMess",
            Protocol::Trojan => "Trojan",
            Protocol::Shadowsocks => "SS",
        }
    }
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Transmission {
    TCP,
    WebSocket,
    GRPC,
    XHTTP,
    HTTPUpgrade,
    SplitHTTP,
}

impl Transmission {
    pub fn as_str(&self) -> &'static str {
        match self {
            Transmission::TCP => "TCP",
            Transmission::WebSocket => "WS",
            Transmission::GRPC => "gRPC",
            Transmission::XHTTP => "XHTTP",
            Transmission::HTTPUpgrade => "HTTPUpgrade",
            Transmission::SplitHTTP => "SplitHTTP",
        }
    }

    pub fn as_type_str(&self) -> &'static str {
        match self {
            Transmission::TCP => "tcp",
            Transmission::WebSocket => "ws",
            Transmission::GRPC => "grpc",
            Transmission::XHTTP => "xhttp",
            Transmission::HTTPUpgrade => "httpupgrade",
            Transmission::SplitHTTP => "splithttp",
        }
    }
}

impl std::fmt::Display for Transmission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Security {
    TLS,
    Reality,
    None,
}

impl Security {
    pub fn as_str(&self) -> &'static str {
        match self {
            Security::TLS => "tls",
            Security::Reality => "reality",
            Security::None => "none",
        }
    }
}

impl std::fmt::Display for Security {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
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
                "www.cloudflare.com",
                "www.google.com",
                "www.microsoft.com",
                "discord.com",
                "telegram.org",
                "www.amazon.com",
                "www.apple.com",
            ],
            paths: vec![
                "/",
                "/ws",
                "/vless",
                "/vmess",
                "/api",
                "/graphql",
            ],
            fingerprints: vec![
                "chrome",
                "firefox",
                "safari",
                "edge",
                "random",
            ],
        }
    }

    pub fn generate_configs(&self, proxy_ip: &str, port: u16) -> Vec<ProxyConfig> {
        let mut configs = Vec::new();
        let mut rng = rand::thread_rng();

        // VLESS configs
        configs.push(self.create_vless_ws_tls(proxy_ip, port, &mut rng));
        configs.push(self.create_vless_grpc_tls(proxy_ip, port, &mut rng));
        configs.push(self.create_vless_tcp_reality(proxy_ip, port, &mut rng));
        
        // VMess configs
        configs.push(self.create_vmess_ws_tls(proxy_ip, port, &mut rng));
        configs.push(self.create_vmess_tcp_tls(proxy_ip, port, &mut rng));
        
        // Trojan configs
        configs.push(self.create_trojan_ws_tls(proxy_ip, port, &mut rng));
        configs.push(self.create_trojan_grpc_tls(proxy_ip, port, &mut rng));
        
        // Shadowsocks config
        configs.push(self.create_shadowsocks(proxy_ip, port, &mut rng));

        configs
    }

    fn create_vless_ws_tls(&self, address: &str, port: u16, rng: &mut impl Rng) -> ProxyConfig {
        let sni = self.snis[rng.gen_range(0..self.snis.len())];
        ProxyConfig {
            protocol: Protocol::VLESS,
            transmission: Transmission::WebSocket,
            address: address.to_string(),
            port,
            id: uuid::Uuid::new_v4().to_string(),
            security: Security::TLS,
            sni: sni.to_string(),
            alpn: vec!["h2".to_string(), "http/1.1".to_string()],
            fingerprint: self.fingerprints[rng.gen_range(0..self.fingerprints.len())].to_string(),
            path: Some(self.paths[rng.gen_range(0..self.paths.len())].to_string()),
            host: Some(sni.to_string()),
            service_name: None,
            public_key: None,
            short_id: None,
        }
    }

    fn create_vless_grpc_tls(&self, address: &str, port: u16, rng: &mut impl Rng) -> ProxyConfig {
        let sni = self.snis[rng.gen_range(0..self.snis.len())];
        ProxyConfig {
            protocol: Protocol::VLESS,
            transmission: Transmission::GRPC,
            address: address.to_string(),
            port,
            id: uuid::Uuid::new_v4().to_string(),
            security: Security::TLS,
            sni: sni.to_string(),
            alpn: vec!["h2".to_string()],
            fingerprint: self.fingerprints[rng.gen_range(0..self.fingerprints.len())].to_string(),
            path: None,
            host: None,
            service_name: Some(format!("grpc{}", rng.gen_range(1000..9999))),
            public_key: None,
            short_id: None,
        }
    }

    fn create_vless_tcp_reality(&self, address: &str, port: u16, rng: &mut impl Rng) -> ProxyConfig {
        let sni = self.snis[rng.gen_range(0..self.snis.len())];
        ProxyConfig {
            protocol: Protocol::VLESS,
            transmission: Transmission::TCP,
            address: address.to_string(),
            port,
            id: uuid::Uuid::new_v4().to_string(),
            security: Security::Reality,
            sni: sni.to_string(),
            alpn: vec!["h2".to_string()],
            fingerprint: self.fingerprints[rng.gen_range(0..self.fingerprints.len())].to_string(),
            path: None,
            host: None,
            service_name: None,
            public_key: Some(self.generate_public_key(rng)),
            short_id: Some(self.generate_short_id(rng)),
        }
    }

    fn create_vmess_ws_tls(&self, address: &str, port: u16, rng: &mut impl Rng) -> ProxyConfig {
        let sni = self.snis[rng.gen_range(0..self.snis.len())];
        ProxyConfig {
            protocol: Protocol::VMess,
            transmission: Transmission::WebSocket,
            address: address.to_string(),
            port,
            id: uuid::Uuid::new_v4().to_string(),
            security: Security::TLS,
            sni: sni.to_string(),
            alpn: vec!["h2".to_string(), "http/1.1".to_string()],
            fingerprint: self.fingerprints[rng.gen_range(0..self.fingerprints.len())].to_string(),
            path: Some(self.paths[rng.gen_range(0..self.paths.len())].to_string()),
            host: Some(sni.to_string()),
            service_name: None,
            public_key: None,
            short_id: None,
        }
    }

    fn create_vmess_tcp_tls(&self, address: &str, port: u16, rng: &mut impl Rng) -> ProxyConfig {
        let sni = self.snis[rng.gen_range(0..self.snis.len())];
        ProxyConfig {
            protocol: Protocol::VMess,
            transmission: Transmission::TCP,
            address: address.to_string(),
            port,
            id: uuid::Uuid::new_v4().to_string(),
            security: Security::TLS,
            sni: sni.to_string(),
            alpn: vec!["h2".to_string(), "http/1.1".to_string()],
            fingerprint: self.fingerprints[rng.gen_range(0..self.fingerprints.len())].to_string(),
            path: None,
            host: None,
            service_name: None,
            public_key: None,
            short_id: None,
        }
    }

    fn create_trojan_ws_tls(&self, address: &str, port: u16, rng: &mut impl Rng) -> ProxyConfig {
        let sni = self.snis[rng.gen_range(0..self.snis.len())];
        ProxyConfig {
            protocol: Protocol::Trojan,
            transmission: Transmission::WebSocket,
            address: address.to_string(),
            port,
            id: uuid::Uuid::new_v4().to_string(),
            security: Security::TLS,
            sni: sni.to_string(),
            alpn: vec!["h2".to_string(), "http/1.1".to_string()],
            fingerprint: self.fingerprints[rng.gen_range(0..self.fingerprints.len())].to_string(),
            path: Some(self.paths[rng.gen_range(0..self.paths.len())].to_string()),
            host: Some(sni.to_string()),
            service_name: None,
            public_key: None,
            short_id: None,
        }
    }

    fn create_trojan_grpc_tls(&self, address: &str, port: u16, rng: &mut impl Rng) -> ProxyConfig {
        let sni = self.snis[rng.gen_range(0..self.snis.len())];
        ProxyConfig {
            protocol: Protocol::Trojan,
            transmission: Transmission::GRPC,
            address: address.to_string(),
            port,
            id: uuid::Uuid::new_v4().to_string(),
            security: Security::TLS,
            sni: sni.to_string(),
            alpn: vec!["h2".to_string()],
            fingerprint: self.fingerprints[rng.gen_range(0..self.fingerprints.len())].to_string(),
            path: None,
            host: None,
            service_name: Some(format!("trojan{}", rng.gen_range(1000..9999))),
            public_key: None,
            short_id: None,
        }
    }

    fn create_shadowsocks(&self, address: &str, port: u16, rng: &mut impl Rng) -> ProxyConfig {
        ProxyConfig {
            protocol: Protocol::Shadowsocks,
            transmission: Transmission::TCP,
            address: address.to_string(),
            port,
            id: self.generate_password(rng),
            security: Security::None,
            sni: String::new(),
            alpn: vec![],
            fingerprint: String::new(),
            path: None,
            host: None,
            service_name: None,
            public_key: None,
            short_id: None,
        }
    }

    fn generate_public_key(&self, rng: &mut impl Rng) -> String {
        let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        general_purpose::STANDARD.encode(&bytes)
    }

    fn generate_short_id(&self, rng: &mut impl Rng) -> String {
        let hex: Vec<char> = "0123456789abcdef".chars().collect();
        (0..8).map(|_| hex[rng.gen_range(0..16)]).collect()
    }

    fn generate_password(&self, rng: &mut impl Rng) -> String {
        let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".chars().collect();
        (0..16).map(|_| chars[rng.gen_range(0..chars.len())]).collect()
    }

    pub fn to_link(&self, config: &ProxyConfig) -> String {
        match config.protocol {
            Protocol::VLESS => self.vless_link(config),
            Protocol::VMess => self.vmess_link(config),
            Protocol::Trojan => self.trojan_link(config),
            Protocol::Shadowsocks => self.ss_link(config),
        }
    }

    fn vless_link(&self, c: &ProxyConfig) -> String {
        let mut params = vec![
            format!("type={}", c.transmission.as_type_str()),
            format!("security={}", c.security.as_str()),
        ];

        if !c.fingerprint.is_empty() {
            params.push(format!("fp={}", c.fingerprint));
        }
        if !c.sni.is_empty() {
            params.push(format!("sni={}", c.sni));
        }
        if !c.alpn.is_empty() {
            params.push(format!("alpn={}", c.alpn.join(",")));
        }
        if let Some(ref path) = c.path {
            params.push(format!("path={}", urlencoding::encode(path)));
        }
        if let Some(ref host) = c.host {
            params.push(format!("host={}", host));
        }
        if let Some(ref sn) = c.service_name {
            params.push(format!("serviceName={}", sn));
        }
        if let Some(ref pk) = c.public_key {
            params.push(format!("pbk={}", pk));
        }
        if let Some(ref sid) = c.short_id {
            params.push(format!("sid={}", sid));
        }

        let name = format!("VLESS_{}_{}", c.transmission.as_str(), c.address);
        format!(
            "vless://{}@{}:{}?{}&encryption=none#{}",
            c.id, c.address, c.port,
            params.join("&"),
            urlencoding::encode(&name)
        )
    }

    fn vmess_link(&self, c: &ProxyConfig) -> String {
        let json = serde_json::json!({
            "v": "2",
            "ps": format!("VMess_{}_{}", c.transmission.as_str(), c.address),
            "add": c.address,
            "port": c.port,
            "id": c.id,
            "aid": "0",
            "scy": "auto",
            "net": c.transmission.as_type_str(),
            "type": "none",
            "host": c.host.clone().unwrap_or_default(),
            "path": c.path.clone().unwrap_or_default(),
            "tls": if c.security == Security::TLS { "tls" } else { "" },
            "sni": c.sni,
            "alpn": c.alpn.join(","),
            "fp": c.fingerprint,
        });
        format!("vmess://{}", general_purpose::STANDARD.encode(json.to_string()))
    }

    fn trojan_link(&self, c: &ProxyConfig) -> String {
        let mut params = vec![
            format!("type={}", c.transmission.as_type_str()),
            "security=tls".to_string(),
            format!("sni={}", c.sni),
        ];

        if !c.fingerprint.is_empty() {
            params.push(format!("fp={}", c.fingerprint));
        }
        if !c.alpn.is_empty() {
            params.push(format!("alpn={}", c.alpn.join(",")));
        }
        if let Some(ref path) = c.path {
            params.push(format!("path={}", urlencoding::encode(path)));
        }
        if let Some(ref host) = c.host {
            params.push(format!("host={}", host));
        }
        if let Some(ref sn) = c.service_name {
            params.push(format!("serviceName={}", sn));
        }

        let name = format!("Trojan_{}_{}", c.transmission.as_str(), c.address);
        format!(
            "trojan://{}@{}:{}?{}#{}",
            c.id, c.address, c.port,
            params.join("&"),
            urlencoding::encode(&name)
        )
    }

    fn ss_link(&self, c: &ProxyConfig) -> String {
        let method = "chacha20-ietf-poly1305";
        let userinfo = format!("{}:{}", method, c.id);
        let encoded = general_purpose::STANDARD.encode(&userinfo);
        format!("ss://{}@{}:{}#SS_{}", encoded, c.address, c.port, urlencoding::encode(&c.address))
    }
}
