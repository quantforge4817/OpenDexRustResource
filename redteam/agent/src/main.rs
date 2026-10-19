//! OpenDex Red Team Agent
//!
//! This agent connects to the OpenDex testnet as a legitimate libp2p peer,
//! then executes adversarial attack scenarios to test network defenses.
//!
//! # Usage
//!
//! ```bash
//! # Connect to honest nodes and run all scenarios
//! cargo run --bin red-team-agent -- --connect /ip4/10.0.1.10/tcp/4001 --all
//!
//! # Run a specific scenario category
//! cargo run --bin red-team-agent -- --connect /ip4/10.0.1.10/tcp/4001 --category protocol
//!
//! # Run a specific scenario
//! cargo run --bin red-team-agent -- --connect /ip4/10.0.1.10/tcp/4001 --scenario PA-001
//! ```

mod protocol_attacks;
mod crypto_attacks;
mod dos_attacks;
mod fiat_attacks;

use anyhow::Result;
use clap::Parser;
use libp2p::{
    gossipsub::{self, IdentTopic, Message as GossipSubMessage},
    identity,
    multiaddr::Protocol,
    noise,
    swarm::NetworkBehaviour,
    tcp, yamux, PeerId, Swarm, Multiaddr,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;
use tracing_subscriber::EnvFilter;

/// OpenDex Red Team Agent
#[derive(Parser, Debug)]
#[command(name = "red-team-agent")]
#[command(about = "Adversarial P2P network testing agent for OpenDex")]
struct Args {
    /// Multiaddrs of honest nodes to connect to (can be specified multiple times)
    #[arg(long = "connect", num_args = 1..)]
    connect: Vec<Multiaddr>,

    /// Run all scenarios
    #[arg(long = "all")]
    all: bool,

    /// Run scenarios from a specific category
    #[arg(long = "category")]
    category: Option<String>,

    /// Run a specific scenario by ID
    #[arg(long = "scenario")]
    scenario: Option<String>,

    /// Number of iterations for each scenario
    #[arg(long = "iterations", default_value = "1")]
    iterations: usize,

    /// Listen port for libp2p
    #[arg(long = "port", default_value = "4000")]
    port: u16,
}

/// Result of a single scenario execution
#[derive(Debug, Serialize, Deserialize)]
struct ScenarioResult {
    scenario_id: String,
    category: String,
    description: String,
    status: ScenarioStatus,
    severity: String,
    duration_ms: u64,
    details: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
enum ScenarioStatus {
    Pass,
    Fail,
    Partial,
    Error,
}

impl std::fmt::Display for ScenarioStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScenarioStatus::Pass => write!(f, "✅ PASS"),
            ScenarioStatus::Fail => write!(f, "❌ FAIL"),
            ScenarioStatus::Partial => write!(f, "⚠️ PARTIAL"),
            ScenarioStatus::Error => write!(f, "💥 ERROR"),
        }
    }
}

/// Libp2p swarm behaviour for the attacker node
#[derive(NetworkBehaviour)]
struct AttackerBehaviour {
    gossipsub: gossipsub::Behaviour,
}

/// Red team agent that manages the libp2p swarm and executes scenarios
struct RedTeamAgent {
    swarm: Swarm<AttackerBehaviour>,
    connected_peers: HashMap<PeerId, Multiaddr>,
    results: Vec<ScenarioResult>,
}

impl RedTeamAgent {
    /// Create a new red team agent
    async fn new(listen_port: u16) -> Result<Self> {
        // Generate identity
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        tracing::info!("Local peer ID: {}", local_peer_id);

        // Setup GossipSub
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .validation_mode(gossipsub::ValidationMode::Permissive)
            .build()
            .expect("Valid GossipSub config");

        let gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_config,
        )?;

        // Build swarm
        let mut swarm = Swarm::builder_v2()
            .with_identity(local_key)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_behaviour(|_| AttackerBehaviour { gossipsub })?
            .with_swarm_config(|c| c.with_idle_timeout(std::time::Duration::from_secs(60)))
            .build();

        // Listen on specified port
        let listen_addr = Multiaddr::empty()
            .with(Protocol::Ip4([0, 0, 0, 0].into()))
            .with(Protocol::Tcp(listen_port));

        swarm.listen_on(listen_addr)?;
        tracing::info!("Listening on port {}", listen_port);

        Ok(Self {
            swarm,
            connected_peers: HashMap::new(),
            results: Vec::new(),
        })
    }

    /// Connect to honest nodes
    async fn connect_to_peers(&mut self, peer_addresses: &[Multiaddr]) -> Result<()> {
        for addr in peer_addresses {
            tracing::info!("Connecting to peer at {}", addr);
            if let Err(e) = self.swarm.dial(addr.clone()) {
                tracing::warn!("Failed to dial {}: {}", addr, e);
            }
        }

        // Wait for connections
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        let peer_count = self.connected_peers.len();
        tracing::info!("Connected to {} peers", peer_count);

        Ok(())
    }

    /// Publish a message to GossipSub
    async fn publish_message(&mut self, topic: &str, data: &[u8]) -> Result<()> {
        let topic = IdentTopic::new(topic.to_string());
        
        if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(
            topic.clone(),
            data.to_vec(),
        ) {
            tracing::error!("Failed to publish message: {}", e);
            return Err(e.into());
        }

        tracing::debug!("Published message to topic: {}", topic);
        Ok(())
    }

    /// Execute a single scenario
    async fn execute_scenario(
        &mut self,
        scenario_id: &str,
        category: &str,
        description: &str,
        severity: &str,
        attack_fn: impl FnOnce(&mut RedTeamAgent) -> tokio::task::JoinHandle<Result<String>>,
    ) -> ScenarioResult {
        let start = Instant::now();
        
        tracing::info!("Executing scenario: {} - {}", scenario_id, description);
        
        let handle = attack_fn(self);
        let result = handle.await;
        
        let duration_ms = start.elapsed().as_millis() as u64;
        
        let (status, details) = match result {
            Ok(Ok(details)) => {
                tracing::info!("Scenario {} passed", scenario_id);
                (ScenarioStatus::Pass, details)
            }
            Ok(Err(e)) => {
                tracing::warn!("Scenario {} failed: {}", scenario_id, e);
                (ScenarioStatus::Fail, e.to_string())
            }
            Err(e) => {
                tracing::error!("Scenario {} error: {}", scenario_id, e);
                (ScenarioStatus::Error, e.to_string())
            }
        };

        let result = ScenarioResult {
            scenario_id: scenario_id.to_string(),
            category: category.to_string(),
            description: description.to_string(),
            status,
            severity: severity.to_string(),
            duration_ms,
            details,
        };

        self.results.push(result.clone());
        result
    }

    /// Run all protocol attack scenarios
    async fn run_protocol_attacks(&mut self, iterations: usize) {
        for i in 0..iterations {
            tracing::info!("Protocol attack iteration {}/{}", i + 1, iterations);
            
            // PA-001: Malformed message injection
            self.execute_scenario(
                "PA-001",
                "Protocol",
                "Malformed message injection",
                "Medium",
                |agent| protocol_attacks::malformed_message_injection(agent),
            ).await;

            // PA-002: Replay attack
            self.execute_scenario(
                "PA-002",
                "Protocol",
                "Replay attack (live network)",
                "High",
                |agent| protocol_attacks::replay_attack(agent),
            ).await;

            // PA-003: Sequence manipulation
            self.execute_scenario(
                "PA-003",
                "Protocol",
                "Sequence manipulation",
                "Medium",
                |agent| protocol_attacks::sequence_manipulation(agent),
            ).await;

            // PA-004: Topic hijacking
            self.execute_scenario(
                "PA-004",
                "Protocol",
                "Topic hijacking",
                "Low",
                |agent| protocol_attacks::topic_hijacking(agent),
            ).await;

            // PA-005: Identity spoofing
            self.execute_scenario(
                "PA-005",
                "Protocol",
                "Identity spoofing",
                "High",
                |agent| protocol_attacks::identity_spoofing(agent),
            ).await;

            // PA-006: Protocol downgrade
            self.execute_scenario(
                "PA-006",
                "Protocol",
                "Protocol downgrade",
                "Medium",
                |agent| protocol_attacks::protocol_downgrade(agent),
            ).await;

            // PA-007: GossipSub flooding
            self.execute_scenario(
                "PA-007",
                "Protocol",
                "GossipSub flooding",
                "High",
                |agent| protocol_attacks::gossipsub_flooding(agent),
            ).await;

            // PA-008: Message size attack
            self.execute_scenario(
                "PA-008",
                "Protocol",
                "Message size attack",
                "Medium",
                |agent| protocol_attacks::message_size_attack(agent),
            ).await;
        }
    }

    /// Run all crypto attack scenarios
    async fn run_crypto_attacks(&mut self, iterations: usize) {
        for i in 0..iterations {
            tracing::info!("Crypto attack iteration {}/{}", i + 1, iterations);
            
            // CA-001: HMAC token replay
            self.execute_scenario(
                "CA-001",
                "Crypto",
                "HMAC token replay",
                "High",
                |agent| crypto_attacks::hmac_token_replay(agent),
            ).await;

            // CA-002: Token timing analysis
            self.execute_scenario(
                "CA-002",
                "Crypto",
                "Token timing analysis",
                "Medium",
                |agent| crypto_attacks::token_timing_analysis(agent),
            ).await;

            // CA-003: Key extraction attempt
            self.execute_scenario(
                "CA-003",
                "Crypto",
                "Key extraction attempt",
                "High",
                |agent| crypto_attacks::key_extraction_attempt(agent),
            ).await;

            // CA-004: Signature malleability
            self.execute_scenario(
                "CA-004",
                "Crypto",
                "Signature malleability",
                "High",
                |agent| crypto_attacks::signature_malleability(agent),
            ).await;

            // CA-005: Key uniqueness test
            self.execute_scenario(
                "CA-005",
                "Crypto",
                "Key uniqueness test",
                "Medium",
                |agent| crypto_attacks::key_uniqueness_test(agent),
            ).await;

            // CA-006: Argon2id bypass
            self.execute_scenario(
                "CA-006",
                "Crypto",
                "Argon2id bypass",
                "High",
                |agent| crypto_attacks::argon2id_bypass(agent),
            ).await;

            // CA-007: DLEQ proof forgery
            self.execute_scenario(
                "CA-007",
                "Crypto",
                "DLEQ proof forgery",
                "Critical",
                |agent| crypto_attacks::dleq_proof_forgery(agent),
            ).await;
        }
    }

    /// Run all DoS attack scenarios
    async fn run_dos_attacks(&mut self, iterations: usize) {
        for i in 0..iterations {
            tracing::info!("DoS attack iteration {}/{}", i + 1, iterations);
            
            // DA-001: Connection exhaustion
            self.execute_scenario(
                "DA-001",
                "DoS",
                "Connection exhaustion",
                "High",
                |agent| dos_attacks::connection_exhaustion(agent),
            ).await;

            // DA-002: State bloat
            self.execute_scenario(
                "DA-002",
                "DoS",
                "State bloat",
                "Medium",
                |agent| dos_attacks::state_bloat(agent),
            ).await;

            // DA-003: GossipSub mesh manipulation
            self.execute_scenario(
                "DA-003",
                "DoS",
                "GossipSub mesh manipulation",
                "Medium",
                |agent| dos_attacks::gossipsub_mesh_manipulation(agent),
            ).await;

            // DA-004: Resource starvation
            self.execute_scenario(
                "DA-004",
                "DoS",
                "Resource starvation",
                "High",
                |agent| dos_attacks::resource_starvation(agent),
            ).await;

            // DA-005: Slowloris attack
            self.execute_scenario(
                "DA-005",
                "DoS",
                "Slowloris attack",
                "Medium",
                |agent| dos_attacks::slowloris_attack(agent),
            ).await;

            // DA-006: DHT pollution
            self.execute_scenario(
                "DA-006",
                "DoS",
                "DHT pollution",
                "Low",
                |agent| dos_attacks::dht_pollution(agent),
            ).await;
        }
    }

    /// Run all fiat attack scenarios
    async fn run_fiat_attacks(&mut self, iterations: usize) {
        for i in 0..iterations {
            tracing::info!("Fiat attack iteration {}/{}", i + 1, iterations);
            
            // FA-001: False payment detection
            self.execute_scenario(
                "FA-001",
                "Fiat",
                "False payment detection",
                "High",
                |agent| fiat_attacks::false_payment_detection(agent),
            ).await;

            // FA-002: Payment reversal
            self.execute_scenario(
                "FA-002",
                "Fiat",
                "Payment reversal",
                "High",
                |agent| fiat_attacks::payment_reversal(agent),
            ).await;

            // FA-003: Race condition exploit
            self.execute_scenario(
                "FA-003",
                "Fiat",
                "Race condition exploit",
                "Critical",
                |agent| fiat_attacks::race_condition_exploit(agent),
            ).await;

            // FA-004: Chargeback simulation
            self.execute_scenario(
                "FA-004",
                "Fiat",
                "Chargeback simulation",
                "High",
                |agent| fiat_attacks::chargeback_simulation(agent),
            ).await;

            // FA-005: 2-of-2 multisig edge case
            self.execute_scenario(
                "FA-005",
                "Fiat",
                "2-of-2 multisig edge case",
                "Critical",
                |agent| fiat_attacks::multisig_edge_case(agent),
            ).await;

            // FA-006: Settlement bypass
            self.execute_scenario(
                "FA-006",
                "Fiat",
                "Settlement bypass",
                "High",
                |agent| fiat_attacks::settlement_bypass(agent),
            ).await;

            // FA-007: Cross-currency confusion
            self.execute_scenario(
                "FA-007",
                "Fiat",
                "Cross-currency confusion",
                "Medium",
                |agent| fiat_attacks::cross_currency_confusion(agent),
            ).await;
        }
    }

    /// Print summary of all results
    fn print_summary(&self) {
        println!("\n========================================");
        println!("Red Team Test Results Summary");
        println!("========================================\n");

        let total = self.results.len();
        let passed = self.results.iter().filter(|r| r.status == ScenarioStatus::Pass).count();
        let failed = self.results.iter().filter(|r| r.status == ScenarioStatus::Fail).count();
        let partial = self.results.iter().filter(|r| r.status == ScenarioStatus::Partial).count();
        let errors = self.results.iter().filter(|r| r.status == ScenarioStatus::Error).count();

        println!("Total scenarios: {}", total);
        println!("Passed: {}", passed);
        println!("Failed: {}", failed);
        println!("Partial: {}", partial);
        println!("Errors: {}", errors);
        println!();

        // Group by category
        let mut categories: HashMap<String, Vec<&ScenarioResult>> = HashMap::new();
        for result in &self.results {
            categories
                .entry(result.category.clone())
                .or_default()
                .push(result);
        }

        for (category, results) in &categories {
            println!("Category: {}", category);
            for result in results {
                println!(
                    "  {} [{}] {} - {} ({:.1}ms)",
                    result.status,
                    result.severity,
                    result.scenario_id,
                    result.description,
                    result.duration_ms as f64
                );
            }
            println!();
        }

        // Print details for failed scenarios
        if failed > 0 || errors > 0 {
            println!("Failed/Error Scenarios:");
            for result in &self.results {
                if result.status == ScenarioStatus::Fail || result.status == ScenarioStatus::Error {
                    println!("  {} - {}", result.scenario_id, result.details);
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    // Validate arguments
    if args.connect.is_empty() && !args.all && args.category.is_none() && args.scenario.is_none() {
        eprintln!("Error: At least one --connect address is required");
        std::process::exit(1);
    }

    tracing::info!("Starting Red Team Agent");

    // Create agent
    let mut agent = RedTeamAgent::new(args.port).await?;

    // Connect to peers if addresses provided
    if !args.connect.is_empty() {
        agent.connect_to_peers(&args.connect).await?;
    }

    // Execute scenarios based on arguments
    if args.all {
        tracing::info!("Running all scenarios");
        agent.run_protocol_attacks(args.iterations).await;
        agent.run_crypto_attacks(args.iterations).await;
        agent.run_dos_attacks(args.iterations).await;
        agent.run_fiat_attacks(args.iterations).await;
    } else if let Some(ref category) = args.category {
        tracing::info!("Running category: {}", category);
        match category.to_lowercase().as_str() {
            "protocol" => agent.run_protocol_attacks(args.iterations).await,
            "crypto" => agent.run_crypto_attacks(args.iterations).await,
            "dos" => agent.run_dos_attacks(args.iterations).await,
            "fiat" => agent.run_fiat_attacks(args.iterations).await,
            _ => {
                eprintln!("Unknown category: {}. Valid categories: protocol, crypto, dos, fiat", category);
                std::process::exit(1);
            }
        }
    } else if let Some(ref scenario_id) = args.scenario {
        tracing::info!("Running scenario: {}", scenario_id);
        // Dispatch to specific scenario based on ID prefix
        if scenario_id.starts_with("PA-") {
            agent.run_protocol_attacks(1).await;
        } else if scenario_id.starts_with("CA-") {
            agent.run_crypto_attacks(1).await;
        } else if scenario_id.starts_with("DA-") {
            agent.run_dos_attacks(1).await;
        } else if scenario_id.starts_with("FA-") {
            agent.run_fiat_attacks(1).await;
        } else {
            eprintln!("Unknown scenario ID: {}. Use PA-, CA-, DA-, or FA- prefix.", scenario_id);
            std::process::exit(1);
        }
    }

    // Print summary
    agent.print_summary();

    Ok(())
}
