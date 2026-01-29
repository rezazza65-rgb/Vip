use serde::{Deserialize, Serialize};
use rand::Rng;
use std::collections::HashMap;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Protocol {
    VLESS,
    VMess,
    Trojan,
    Shadowsocks,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Transmission {
    TCP,
    WebSocket,
    GRPC,
    XHTTP,
    HTTPUpgrade,
    SplitHTTP,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Security {
    TLS,
    Reality,
    None,
}

pub struct ConfigGenerator {
    popular_snis: Vec<String>,
    popular_paths: Vec<String>,
    fingerprints: Vec<String>,
    alpn_combinations: Vec<Vec<String>>,
}

impl ConfigGenerator {
    pub fn new() -> Self {
        Self {
            popular_snis: vec![
                "www.speedtest.net".to_string(),
                "www.yahoo.com".to_string(),
                "www.cloudflare.com".to_string(),
                "www.google.com".to_string(),
                "www.microsoft.com".to_string(),
                "www.bing.com".to_string(),
                "www.booking.com".to_string(),
                "www.cisco.com".to_string(),
                "www.wikipedia.org".to_string(),
                "discord.com".to_string(),
                "telegram.org".to_string(),
                "www.ubuntu.com".to_string(),
                "www.nvidia.com".to_string(),
                "www.amd.com".to_string(),
                "aws.amazon.com".to_string(),
            ],
            popular_paths: vec![
                "/".to_string(),
                "/ws".to_string(),
                "/vless".to_string(),
                "/vmess".to_string(),
                "/api".to_string(),
                "/download".to_string(),
                "/upgrade".to_string(),
                "/socket".to_string(),
                "/graphql".to_string(),
                "/cdn-cgi/trace".to_string(),
            ],
            fingerprints: vec![
                "chrome".to_string(),
                "firefox".to_string(),
                "safari".to_string(),
                "ios".to_string(),
                "android".to_string(),
                "edge".to_string(),
                "360".to_string(),
                "qq".to_string(),
                "random".to_string(),
                "randomized".to_string(),
            ],
            alpn_combinations: vec![
                vec!["h2".to_string(), "http/1.1".to_string()],
                vec!["h2".to_string()],
                vec!["http/1.1".to_string()],
                vec!["h3".to_string()],
            ],
        }
    }

    pub fn generate_configs(&self, proxy_ip: &str, port: u16) -> Vec<ProxyConfig> {
        let mut configs = Vec::new();
        let mut rng = rand::thread_rng();

        // Priority configurations (most effective combinations)
        let priority_combinations = vec![
            (Protocol::VLESS, Transmission::XHTTP, Security::Reality),
            (Protocol::VLESS, Transmission::GRPC, Security::Reality),
            (Protocol::VLESS, Transmission::WebSocket, Security::TLS),
            (Protocol::VLESS, Transmission::HTTPUpgrade, Security::Reality),
            (Protocol::VLESS, Transmission::SplitHTTP, Security::Reality),
            (Protocol::VMess, Transmission::WebSocket, Security::TLS),
            (Protocol::VMess, Transmission::GRPC, Security::TLS),
            (Protocol::Trojan, Transmission::WebSocket, Security::TLS),
            (Protocol::Trojan, Transmission::GRPC, Security::TLS),
        ];

        for (protocol, transmission, security) in priority_combinations {
            let config = self.create_config(
                proxy_ip,
                port,
                protocol,
                transmission,
                security,
                &mut rng,
            );
            configs.push(config);
        }

        // Additional variations for comprehensive coverage
        for protocol in &[Protocol::VLESS, Protocol::VMess, Protocol::Trojan] {
            for transmission in &[Transmission::TCP, Transmission::WebSocket] {
                let security = if matches!(protocol, Protocol::VLESS) {
                    Security::Reality
                } else {
                    Security::TLS
                };

                let config = self.create_config(
                    proxy_ip,
                    port,
                    protocol.clone(),
                    transmission.clone(),
                    security,
                    &mut rng,
                );
                configs.push(config);
            }
        }

        configs
    }

    fn create_config(
        &self,
        address: &str,
        port: u16,
        protocol: Protocol,
        transmission: Transmission,
        security: Security,
        rng: &mut impl Rng,
    ) -> ProxyConfig {
        let sni = self.popular_snis[rng.gen_range(0..self.popular_snis.len())].clone();
        let alpn = self.alpn_combinations[rng.gen_range(0..self.alpn_combinations.len())].clone();
        let fingerprint = self.fingerprints[rng.gen_range(0..self.fingerprints.len())].clone();
        
        let id = self.generate_uuid();
        
        let (path, host, service_name) = match transmission {
            Transmission::WebSocket | Transmission::HTTPUpgrade | Transmission::SplitHTTP => {
                let path = Some(self.popular_paths[rng.gen_range(0..self.popular_paths.len())].clone());
                let host = Some(sni.clone());
                (path, host, None)
            }
            Transmission::GRPC => {
                let service_name = Some(format!("grpc{}", rng.gen_range(1000..9999)));
                (None, None, service_name)
            }
            Transmission::XHTTP => {
                let path = Some("/xhttp".to_string());
                let host = Some(sni.clone());
                (path, host, None)
            }
            Transmission::TCP => (None, None, None),
        };

        let (public_key, short_id) = if matches!(security, Security::Reality) {
            (
                Some(self.generate_public_key()),
                Some(self.generate_short_id(rng)),
            )
        } else {
            (None, None)
        };

        ProxyConfig {
            protocol,
            transmission,
            address: address.to_string(),
            port,
            id,
            security,
            sni,
            alpn,
            fingerprint,
            path,
            host,
            service_name,
            public_key,
            short_id,
        }
    }

    fn generate_uuid(&self) -> String {
        use uuid::Uuid;
        Uuid::new_v4().to_string()
    }

    fn generate_public_key(&self) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        base64::encode(&bytes)
    }

    fn generate_short_id(&self, rng: &mut impl Rng) -> String {
        let hex_chars = "0123456789abcdef";
        (0..8)
            .map(|_| {
                let idx = rng.gen_range(0..hex_chars.len());
                hex_chars.chars().nth(idx).unwrap()
            })
            .collect()
    }

    pub fn to_subscription_link(&self, config: &ProxyConfig) -> String {
        match config.protocol {
            Protocol::VLESS => self.vless_to_link(config),
            Protocol::VMess => self.vmess_to_link(config),
            Protocol::Trojan => self.trojan_to_link(config),
            Protocol::Shadowsocks => self.ss_to_link(config),
        }
    }

    fn vless_to_link(&self, config: &ProxyConfig) -> String {
        let transmission_type = match config.transmission {
            Transmission::TCP => "tcp",
            Transmission::WebSocket => "ws",
            Transmission::GRPC => "grpc",
            Transmission::XHTTP => "xhttp",
            Transmission::HTTPUpgrade => "httpupgrade",
            Transmission::SplitHTTP => "splithttp",
        };

        let security_type = match config.security {
            Security::TLS => "tls",
            Security::Reality => "reality",
            Security::None => "none",
        };

        let mut params = vec![
            format!("type={}", transmission_type),
            format!("security={}", security_type),
            format!("fp={}", config.fingerprint),
            format!("sni={}", config.sni),
        ];

        if !config.alpn.is_empty() {
            params.push(format!("alpn={}", config.alpn.join(",")));
        }

        if let Some(path) = &config.path {
            params.push(format!("path={}", urlencoding::encode(path)));
        }

        if let Some(host) = &config.host {
            params.push(format!("host={}", host));
        }

        if let Some(service_name) = &config.service_name {
            params.push(format!("serviceName={}", service_name));
        }

        if let Some(public_key) = &config.public_key {
            params.push(format!("pbk={}", public_key));
        }

        if let Some(short_id) = &config.short_id {
            params.push(format!("sid={}", short_id));
        }

        let remark = format!(
            "{}_{}_{}",
            config.protocol.to_string(),
            transmission_type,
            config.address
        );

        format!(
            "vless://{}@{}:{}?{}&{}#{}",
            config.id,
            config.address,
            config.port,
            params.join("&"),
            format!("encryption=none"),
            urlencoding::encode(&remark)
        )
    }

    fn vmess_to_link(&self, config: &ProxyConfig) -> String {
        let vmess_json = serde_json::json!({
            "v": "2",
            "ps": format!("VMess_{}_{}", config.transmission.to_string(), config.address),
            "add": config.address,
            "port": config.port,
            "id": config.id,
            "aid": "0",
            "scy": "auto",
            "net": match config.transmission {
                Transmission::TCP => "tcp",
                Transmission::WebSocket => "ws",
                Transmission::GRPC => "grpc",
                Transmission::HTTPUpgrade => "httpupgrade",
                _ => "tcp",
            },
            "type": "none",
            "host": config.host.clone().unwrap_or_default(),
            "path": config.path.clone().unwrap_or_default(),
            "tls": if matches!(config.security, Security::TLS) { "tls" } else { "" },
            "sni": config.sni,
            "alpn": config.alpn.join(","),
            "fp": config.fingerprint,
        });

        format!("vmess://{}", base64::encode(vmess_json.to_string()))
    }

    fn trojan_to_link(&self, config: &ProxyConfig) -> String {
        let transmission_type = match config.transmission {
            Transmission::WebSocket => "ws",
            Transmission::GRPC => "grpc",
            _ => "tcp",
        };

        let mut params = vec![
            format!("type={}", transmission_type),
            format!("security=tls"),
            format!("sni={}", config.sni),
            format!("fp={}", config.fingerprint),
        ];

        if !config.alpn.is_empty() {
            params.push(format!("alpn={}", config.alpn.join(",")));
        }

        if let Some(path) = &config.path {
            params.push(format!("path={}", urlencoding::encode(path)));
        }

        if let Some(host) = &config.host {
            params.push(format!("host={}", host));
        }

        let remark = format!("Trojan_{}_{}", transmission_type, config.address);

        format!(
            "trojan://{}@{}:{}?{}#{}",
            config.id,
            config.address,
            config.port,
            params.join("&"),
            urlencoding::encode(&remark)
        )
    }

    fn ss_to_link(&self, config: &ProxyConfig) -> String {
        let method = "chacha20-ietf-poly1305";
        let password = &config.id[..16];
        let userinfo = format!("{}:{}", method, password);
        let encoded = base64::encode(&userinfo);
        
        format!(
            "ss://{}@{}:{}#SS_{}",
            encoded,
            config.address,
            config.port,
            urlencoding::encode(&config.address)
        )
    }
}

impl Protocol {
    fn to_string(&self) -> &str {
        match self {
            Protocol::VLESS => "VLESS",
            Protocol::VMess => "VMess",
            Protocol::Trojan => "Trojan",
            Protocol::Shadowsocks => "SS",
        }
    }
}

impl Transmission {
    fn to_string(&self) -> &str {
        match self {
            Transmission::TCP => "TCP",
            Transmission::WebSocket => "WS",
            Transmission::GRPC => "gRPC",
            Transmission::XHTTP => "XHTTP",
            Transmission::HTTPUpgrade => "HTTPUpgrade",
            Transmission::SplitHTTP => "SplitHTTP",
        }
    }
}
