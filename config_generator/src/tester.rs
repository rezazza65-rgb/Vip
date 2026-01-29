use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;

use crate::generator::{ProxyConfig, Protocol, Security};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub config: ProxyConfig,
    pub is_working: bool,
    pub response_time_ms: Option<u64>,
    pub error_message: Option<String>,
    pub test_timestamp: String,
}

#[derive(Clone)]
pub struct ConfigTester {
    timeout_seconds: u64,
    max_concurrent: usize,
}

impl ConfigTester {
    pub fn new(timeout_seconds: u64, max_concurrent: usize) -> Self {
        Self {
            timeout_seconds,
            max_concurrent,
        }
    }

    pub async fn test_configs(&self, configs: Vec<ProxyConfig>) -> Vec<TestResult> {
        let mut results = Vec::new();
        let mut tasks = Vec::new();

        for config in configs {
            let tester = self.clone();
            let task = tokio::spawn(async move { tester.test_single_config(config).await });
            tasks.push(task);

            if tasks.len() >= self.max_concurrent {
                let chunk_results: Vec<TestResult> = futures::future::join_all(tasks.drain(..))
                    .await
                    .into_iter()
                    .filter_map(|r| r.ok())
                    .collect();
                results.extend(chunk_results);
            }
        }

        let remaining_results: Vec<TestResult> = futures::future::join_all(tasks)
            .await
            .into_iter()
            .filter_map(|r| r.ok())
            .collect();
        results.extend(remaining_results);

        results
    }

    async fn test_single_config(&self, config: ProxyConfig) -> TestResult {
        let start = std::time::Instant::now();
        let timestamp = chrono::Utc::now().to_rfc3339();

        let tcp_result = self.test_tcp_connection(&config).await;

        match tcp_result {
            Ok(()) => {
                let protocol_result = self.test_protocol_handshake(&config).await;
                let elapsed = start.elapsed().as_millis() as u64;

                match protocol_result {
                    Ok(()) => TestResult {
                        config,
                        is_working: true,
                        response_time_ms: Some(elapsed),
                        error_message: None,
                        test_timestamp: timestamp,
                    },
                    Err(e) => TestResult {
                        config,
                        is_working: false,
                        response_time_ms: Some(elapsed),
                        error_message: Some(e),
                        test_timestamp: timestamp,
                    },
                }
            }
            Err(e) => TestResult {
                config,
                is_working: false,
                response_time_ms: None,
                error_message: Some(e),
                test_timestamp: timestamp,
            },
        }
    }

    async fn test_tcp_connection(&self, config: &ProxyConfig) -> Result<(), String> {
        let addr = format!("{}:{}", config.address, config.port);
        let timeout_duration = Duration::from_secs(self.timeout_seconds);

        match timeout(timeout_duration, tokio::net::TcpStream::connect(&addr)).await {
            Ok(Ok(_stream)) => Ok(()),
            Ok(Err(e)) => Err(format!("TCP connection failed: {}", e)),
            Err(_) => Err("TCP connection timeout".to_string()),
        }
    }

    async fn test_protocol_handshake(&self, config: &ProxyConfig) -> Result<(), String> {
        match config.protocol {
            Protocol::VLESS => self.test_vless_handshake(config).await,
            Protocol::VMess => self.test_vmess_handshake(config).await,
            Protocol::Trojan => self.test_trojan_handshake(config).await,
            Protocol::Shadowsocks => Ok(()),
        }
    }

    async fn test_vless_handshake(&self, config: &ProxyConfig) -> Result<(), String> {
        let timeout_duration = Duration::from_secs(self.timeout_seconds);

        match &config.security {
            Security::TLS | Security::Reality => {
                match timeout(
                    timeout_duration,
                    self.test_tls_handshake(&config.address, config.port, &config.sni),
                )
                .await
                {
                    Ok(Ok(())) => Ok(()),
                    Ok(Err(e)) => Err(format!("TLS handshake failed: {}", e)),
                    Err(_) => Err("TLS handshake timeout".to_string()),
                }
            }
            Security::None => Ok(()),
        }
    }

    async fn test_vmess_handshake(&self, config: &ProxyConfig) -> Result<(), String> {
        if config.security == Security::TLS {
            self.test_tls_handshake(&config.address, config.port, &config.sni)
                .await
                .map_err(|e| format!("VMess TLS handshake failed: {}", e))
        } else {
            Ok(())
        }
    }

    async fn test_trojan_handshake(&self, config: &ProxyConfig) -> Result<(), String> {
        self.test_tls_handshake(&config.address, config.port, &config.sni)
            .await
            .map_err(|e| format!("Trojan TLS handshake failed: {}", e))
    }

    async fn test_tls_handshake(
        &self,
        address: &str,
        port: u16,
        sni: &str,
    ) -> Result<(), String> {
        use tokio_native_tls::native_tls::TlsConnector;
        use tokio_native_tls::TlsConnector as TokioTlsConnector;

        let connector = TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|e| format!("TLS connector build failed: {}", e))?;

        let connector = TokioTlsConnector::from(connector);

        let addr = format!("{}:{}", address, port);
        let stream = tokio::net::TcpStream::connect(&addr)
            .await
            .map_err(|e| format!("TCP connect failed: {}", e))?;

        let _ = connector
            .connect(sni, stream)
            .await
            .map_err(|e| format!("TLS connect failed: {}", e))?;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestStatistics {
    pub total_configs: usize,
    pub working_configs: usize,
    pub failed_configs: usize,
    pub success_rate: f64,
    pub average_response_time_ms: f64,
    pub fastest_response_time_ms: Option<u64>,
    pub slowest_response_time_ms: Option<u64>,
}

impl Default for TestStatistics {
    fn default() -> Self {
        Self {
            total_configs: 0,
            working_configs: 0,
            failed_configs: 0,
            success_rate: 0.0,
            average_response_time_ms: 0.0,
            fastest_response_time_ms: None,
            slowest_response_time_ms: None,
        }
    }
}

impl TestStatistics {
    pub fn from_results(results: &[TestResult]) -> Self {
        if results.is_empty() {
            return Self::default();
        }

        let total = results.len();
        let working = results.iter().filter(|r| r.is_working).count();
        let failed = total - working;

        let response_times: Vec<u64> = results
            .iter()
            .filter_map(|r| r.response_time_ms)
            .collect();

        let avg_response_time = if !response_times.is_empty() {
            response_times.iter().sum::<u64>() as f64 / response_times.len() as f64
        } else {
            0.0
        };

        let fastest = response_times.iter().min().copied();
        let slowest = response_times.iter().max().copied();

        let success_rate = if total > 0 {
            (working as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        Self {
            total_configs: total,
            working_configs: working,
            failed_configs: failed,
            success_rate,
            average_response_time_ms: avg_response_time,
            fastest_response_time_ms: fastest,
            slowest_response_time_ms: slowest,
        }
    }
}
