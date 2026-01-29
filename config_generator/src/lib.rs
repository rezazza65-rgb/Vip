// Config Generator Library

pub mod generator;
pub mod output;
pub mod tester;

// Re-export main structs and functions
pub use generator::{ConfigGenerator, ProxyConfig, Protocol, Transmission, Security};
pub use output::{OutputGenerator, SubscriptionManager};
pub use tester::ConfigTester;

// ProxyInfo struct
#[derive(Debug, Clone)]
pub struct ProxyInfo {
    pub ip: String,
    pub port: u16,
    pub location: String,
    pub response_time: u32,
}
