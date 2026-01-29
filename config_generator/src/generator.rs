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
    popular_snis: Vec<String>,
    popular_paths: Vec<String>,
    fingerprints: Vec<String>,
    alpn_combinations: Vec<Vec<String>>,
}

impl Default for ConfigGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigGenerator {
    pub fn new() -> Self {
        Self {
            popular_snis: vec![
                "www.speedtest.net".into(),
                "www.yahoo.com".into(),
                "www.cloudflare.com".into(),
                "www.google.com".into(),
                "www.microsoft.com".into(),
                "www.bing.com".into(),
                "www.booking.com".into(),
                "www.cisco.com".into(),
                "www.wikipedia.org".into(),
                "discord.com".into(),
                "telegram.org".into(),
                "www.ubuntu.com".into(),
                "www.nvidia.com".into(),
                "www.amd.com".into(),
                "aws.amazon.com".into(),
            ],
            popular_paths: vec![
                "/".into(),
                "/ws".into(),
                "/vless".into(),
                "/vmess".into(),
                "/api".into(),
                "/download".into(),
                "/upgrade".into(),
                "/socket".into(),
                "/graphql".into(),
                "/cdn-cgi/trace".into(),
            ],
            fingerprints: vec![
                "chrome".into(),
                "firefox".into(),
                "safari".into(),
                "ios".into(),
                "android".into(),
                "edge".into(),
                "360".into(),
                "qq".into(),
                "random".into(),
                "randomized".into(),
            ],
            alpn_combinations: vec![
                vec!["h2".into(), "http/1.1".into()],
                vec!["h2".into()],
                vec!["http/1.1".into()],
                vec!["h3".into()],
            ],
        }
    }

    pub fn generate_configs(&self, proxy_ip: &str, port: u16) -> Vec<ProxyConfig> {
        let mut configs = Vec::new();
        let mut rng = rand::thread_rng();

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

        for protocol in &[Protocol::VLESS, Protocol::VMess, Protocol::Trojan] {
            for transmission in &[Transmission::TCP, Transmission::WebSocket] {
                let security = if *protocol == Protocol::VLESS {
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
                let path =
                    Some(self.popular_paths[rng.gen_range(0..self.popular_paths.len())].clone());
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

        let (public_key, short_id) = if security == Security::Reality {
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
        uuid::Uuid::new_v4().to_string()
    }

    fn generate_public_key(&self) -> String {
        let mut rng = rand::thread_rng();
        let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        general_purpose::STANDARD.encode(&bytes)
    }

    fn generate_short_id(&self, rng: &mut impl Rng) -> String {
        let hex_chars: Vec<char> = "0123456789abcdef".chars().collect();
        (0..8)
            .map(|_| hex_chars[rng.gen_range(0..hex_chars.len())])
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
        let transmission_type = config.transmission.as_type_str();
        let security_type = config.security.as_str();

        let mut params = vec![
            format!("type={}", transmission_type),
            format!("security={}", security_type),
            format!("fp={}", config.fingerprint),
            format!("sni={}", config.sni),
        ];

        if !config.alpn.is_empty() {
            params.push(format!("alpn={}", config.alpn.join(",")));
        }

        if let Some(ref path) = config.path {
            params.push(format!("path={}", urlencoding::encode(path)));
        }

        if let Some(ref host) = config.host {
            params.push(format!("host={}", host));
        }

        if let Some(ref service_name) = config.service_name {
            params.push(format!("serviceName={}", service_name));
        }

        if let Some(ref public_key) = config.public_key {
            params.push(format!("pbk={}", public_key));
        }

        if let Some(ref short_id) = config.short_id {
            params.push(format!("sid={}", short_id));
        }

        let remark = format!(
            "{}_{}_{}",
            config.protocol.as_str(),
            transmission_type,
            config.address
        );

        format!(
            "vless://{}@{}:{}?{}&encryption=none#{}",
            config.id,
            config.address,
            config.port,
            params.join("&"),
            urlencoding::encode(&remark)
        )
    }

    fn vmess_to_link(&self, config: &ProxyConfig) -> String {
        let vmess_json = serde_json::json!({
            "v": "2",
            "ps": format!("VMess_{}_{}", config.transmission.as_str(), config.address),
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
            "tls": if config.security == Security::TLS { "tls" } else { "" },
            "sni": config.sni,
            "alpn": config.alpn.join(","),
            "fp": config.fingerprint,
        });

        format!(
            "vmess://{}",
            general_purpose::STANDARD.encode(vmess_json.to_string())
        )
    }

    fn trojan_to_link(&self, config: &ProxyConfig) -> String {
        let transmission_type = match config.transmission {
            Transmission::WebSocket => "ws",
            Transmission::GRPC => "grpc",
            _ => "tcp",
        };

        let mut params = vec![
            format!("type={}", transmission_type),
            "security=tls".to_string(),
            format!("sni={}", config.sni),
            format!("fp={}", config.fingerprint),
        ];

        if !config.alpn.is_empty() {
            params.push(format!("alpn={}", config.alpn.join(",")));
        }

        if let Some(ref path) = config.path {
            params.push(format!("path={}", urlencoding::encode(path)));
        }

        if let Some(ref host) = config.host {
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
        let password = if config.id.len() >= 16 {
            &config.id[..16]
        } else {
            &config.id
        };
        let userinfo = format!("{}:{}", method, password);
        let encoded = general_purpose::STANDARD.encode(&userinfo);

        format!(
            "ss://{}@{}:{}#SS_{}",
            encoded,
            config.address,
            config.port,
            urlencoding::encode(&config.address)
        )
    }
}
