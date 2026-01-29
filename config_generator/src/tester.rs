use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;

use crate::generator::ProxyConfig;

#[derive(Debug, Clone)]
pub struct TestResult {
    pub config: ProxyConfig,
    pub is_working: bool,
    pub response_time_ms: Option<u64>,
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

    pub async fn test_all(&self, configs: Vec<ProxyConfig>) -> Vec<TestResult> {
        let total = configs.len();
        let mut results = Vec::new();
        let mut tasks = Vec::new();

        for config in configs {
            let tester = self.clone();
            tasks.push(tokio::spawn(async move { tester.test_one(config).await }));

            if tasks.len() >= self.concurrent {
                let batch: Vec<TestResult> = futures::future::join_all(tasks.drain(..))
                    .await
                    .into_iter()
                    .filter_map(|r| r.ok())
                    .collect();
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
        let addr = format!("{}:{}", config.address, config.port);

        let result = timeout(
            Duration::from_secs(self.timeout_secs),
            tokio::net::TcpStream::connect(&addr)
        ).await;

        match result {
            Ok(Ok(_)) => {
                // TLS test
                if let Ok(_) = self.test_tls(&config).await {
                    TestResult {
                        config,
                        is_working: true,
                        response_time_ms: Some(start.elapsed().as_millis() as u64),
                    }
                } else {
                    TestResult {
                        config,
                        is_working: false,
                        response_time_ms: Some(start.elapsed().as_millis() as u64),
                    }
                }
            }
            _ => TestResult {
                config,
                is_working: false,
                response_time_ms: None,
            },
        }
    }

    async fn test_tls(&self, config: &ProxyConfig) -> Result<(), String> {
        use tokio_native_tls::native_tls::TlsConnector;
        use tokio_native_tls::TlsConnector as TokioTls;

        let connector = TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .danger_accept_invalid_hostnames(true)
            .build()
            .map_err(|e| e.to_string())?;

        let connector = TokioTls::from(connector);
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TestStatistics {
    pub total_configs: usize,
    pub working_configs: usize,
    pub failed_configs: usize,
    pub success_rate: f64,
    pub average_response_time_ms: f64,
    pub fastest_response_time_ms: Option<u64>,
    pub slowest_response_time_ms: Option<u64>,
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
