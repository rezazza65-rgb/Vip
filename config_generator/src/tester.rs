use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;

use crate::generator::{ProxyConfig, Protocol, Security};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub config: ProxyConfig,
    pub is_working: bool,
    pub response_time_ms: Option<u64>,
    pub error: Option<String>,
}

#[derive(Clone)]
pub struct ConfigTester {
    timeout_secs: u64,
    concurrent: usize,
}

impl ConfigTester {
    pub fn new(timeout_secs: u64, concurrent: usize) -> Self {
        Self { timeout_secs, concurrent }
    }

    pub async fn test_configs(&self, configs: Vec<ProxyConfig>) -> Vec<TestResult> {
        let total = configs.len();
        println!("   Testing {} configs ({}s timeout)...", total, self.timeout_secs);

        let mut results = Vec::new();
        let mut tasks = Vec::new();
        let mut done = 0;

        for config in configs {
            let tester = self.clone();
            let task = tokio::spawn(async move { tester.test_one(config).await });
            tasks.push(task);

            if tasks.len() >= self.concurrent {
                let batch: Vec<TestResult> = futures::future::join_all(tasks.drain(..))
                    .await
                    .into_iter()
                    .filter_map(|r| r.ok())
                    .collect();

                done += batch.len();
                let working = batch.iter().filter(|r| r.is_working).count();
                println!("   Progress: {}/{} ({} working)", done, total, working);
                results.extend(batch);
            }
        }

        // Remaining
        if !tasks.is_empty() {
            let batch: Vec<TestResult> = futures::future::join_all(tasks)
                .await
                .into_iter()
                .filter_map(|r| r.ok())
                .collect();
            results.extend(batch);
        }

        results
    }

    async fn test_one(&self, config: ProxyConfig) -> TestResult {
        let start = std::time::Instant::now();

        // TCP test
        let addr = format!("{}:{}", config.address, config.port);
        let tcp_result = timeout(
            Duration::from_secs(self.timeout_secs),
            tokio::net::TcpStream::connect(&addr)
        ).await;

        match tcp_result {
            Ok(Ok(_)) => {
                // TLS test if needed
                if config.security == Security::TLS || config.security == Security::Reality {
                    match self.test_tls(&config).await {
                        Ok(_) => TestResult {
                            config,
                            is_working: true,
                            response_time_ms: Some(start.elapsed().as_millis() as u64),
                            error: None,
                        },
                        Err(e) => TestResult {
                            config,
                            is_working: false,
                            response_time_ms: Some(start.elapsed().as_millis() as u64),
                            error: Some(e),
                        },
                    }
                } else {
                    TestResult {
                        config,
                        is_working: true,
                        response_time_ms: Some(start.elapsed().as_millis() as u64),
                        error: None,
                    }
                }
            }
            Ok(Err(e)) => TestResult {
                config,
                is_working: false,
                response_time_ms: None,
                error: Some(format!("TCP: {}", e)),
            },
            Err(_) => TestResult {
                config,
                is_working: false,
                response_time_ms: None,
                error: Some("Timeout".to_string()),
            },
        }
    }

    async fn test_tls(&self, config: &ProxyConfig) -> Result<(), String> {
        use tokio_native_tls::native_tls::TlsConnector;
        use tokio_native_tls::TlsConnector as TokioTlsConnector;

        let connector = TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .danger_accept_invalid_hostnames(true)
            .build()
            .map_err(|e| e.to_string())?;

        let connector = TokioTlsConnector::from(connector);
        let addr = format!("{}:{}", config.address, config.port);

        let stream = tokio::net::TcpStream::connect(&addr)
            .await
            .map_err(|e| e.to_string())?;

        connector.connect(&config.sni, stream)
            .await
            .map_err(|e| e.to_string())?;

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
        
        let times: Vec<u64> = results.iter()
            .filter(|r| r.is_working)
            .filter_map(|r| r.response_time_ms)
            .collect();

        let avg = if times.is_empty() { 0.0 } else {
            times.iter().sum::<u64>() as f64 / times.len() as f64
        };

        Self {
            total_configs: total,
            working_configs: working,
            failed_configs: total - working,
            success_rate: (working as f64 / total as f64) * 100.0,
            average_response_time_ms: avg,
            fastest_response_time_ms: times.iter().min().copied(),
            slowest_response_time_ms: times.iter().max().copied(),
        }
    }
}
