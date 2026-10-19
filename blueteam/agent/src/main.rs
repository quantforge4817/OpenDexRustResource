//! OpenDex Blue Team Observer Agent
//!
//! This agent connects to the OpenDex testnet as an observer node,
//! monitoring network activity to detect and validate defenses against
//! red team attacks. It subscribes to all GossipSub topics and validates
//! that defenses are working correctly.
//!
//! # Usage
//!
//! ```bash
//! # Connect to honest nodes and validate all defenses
//! cargo run -p blueteam-agent -- --connect /ip4/10.0.1.10/tcp/4001 --all
//!
//! # Validate a specific defense category
//! cargo run -p blueteam-agent -- --connect /ip4/10.0.1.10/tcp/4001 --category protocol
//!
//! # Validate a specific defense scenario
//! cargo run -p blueteam-agent -- --connect /ip4/10.0.1.10/tcp/4001 --scenario BD-001
//! ```

mod protocol_defenses;
mod crypto_defenses;
mod dos_defenses;
mod fiat_defenses;
mod transport_defenses;

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

/// OpenDex Blue Team Observer Agent
#[derive(Parser, Debug)]
#[command(name = "blueteam-agent")]
#[command(about = "Defense validation and monitoring agent for OpenDex")]
struct Args {
    /// Multiaddrs of honest nodes to connect to (can be specified multiple times)
    #[arg(long = "connect", num_args = 1..)]
    connect: Vec<Multiaddr>,

    /// Validate all defense scenarios
    #[arg(long = "all")]
    all: bool,

    /// Validate scenarios from a specific category
    #[arg(long = "category")]
    category: Option<String>,

    /// Validate a specific scenario by ID
    #[arg(long = "scenario")]
    scenario: Option<String>,

    /// Number of iterations for each scenario
    #[arg(long = "iterations", default_value = "1")]
    iterations: usize,

    /// Listen port for libp2p
    #[arg(long = "port", default_value = "5000")]
    port: u16,

    /// Observation duration in seconds (for passive monitoring mode)
    #[arg(long = "observe-duration", default_value = "60")]
    observe_duration: u64,
}

/// Result of a single defense validation
#[derive(Debug, Serialize, Deserialize)]
struct ValidationResult {
    scenario_id: String,
    category: String,
    description: String,
    status: ValidationStatus,
    severity: String,
    duration_ms: u64,
    details: String,
    metrics_collected: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
enum ValidationStatus {
    Pass,
    Fail,
    Partial,
    Error,
}

impl std::fmt::Display for ValidationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationStatus::Pass => write!(f, "✅ PASS"),
            ValidationStatus::Fail => write!(f, "❌ FAIL"),
            ValidationStatus::Partial => write!(f, "⚠️ PARTIAL"),
            ValidationStatus::Error => write!(f, "❓ ERROR"),
        }
    }
}

/// Libp2p swarm behaviour for the observer node
#[derive(NetworkBehaviour)]
struct ObserverBehaviour {
    gossipsub: gossipsub::Behaviour,
}

/// Metrics collected during observation
#[derive(Debug, Default)]
struct ObservationMetrics {
    messages_received: u64,
    messages_rejected: u64,
    messages_accepted: u64,
    replay_detections: u64,
    auth_failures: u64,
    rate_limiter_activations: u64,
    connection_count: u64,
    sequence_violations: u64,
    invalid_transitions: u64,
}

/// Blue team observer agent that manages the libp2p swarm and validates defenses
struct BlueTeamAgent {
    swarm: Swarm<ObserverBehaviour>,
    connected_peers: HashMap<PeerId, Multiaddr>,
    results: Vec<ValidationResult>,
    metrics: ObservationMetrics,
}

impl BlueTeamAgent {
    /// Create a new blue team observer agent
    async fn new(listen_port: u16) -> Result<Self> {
        // Generate identity
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        tracing::info!("Blue Team Observer peer ID: {}", local_peer_id);

        // Setup GossipSub in passive observation mode
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .validation_mode(gossipsub::ValidationMode::Permissive)
            .mesh_n(5)
            .mesh_n_low(3)
            .mesh_n_high(7)
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
            .with_behaviour(|_| ObserverBehaviour { gossipsub })?
            .with_swarm_config(|c| c.with_idle_timeout(std::time::Duration::from_secs(120)))
            .build();

        // Listen on specified port
        let listen_addr = Multiaddr::empty()
            .with(Protocol::Ip4([0, 0, 0, 0].into()))
            .with(Protocol::Tcp(listen_port));

        swarm.listen_on(listen_addr)?;
        tracing::info!("Blue Team Observer listening on port {}", listen_port);

        Ok(Self {
            swarm,
            connected_peers: HashMap::new(),
            results: Vec::new(),
            metrics: ObservationMetrics::default(),
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

    /// Subscribe to all topics for observation
    fn subscribe_all_topics(&mut self) {
        let topics = vec![
            "opendex-btc-xmr",
            "opendex-orders",
            "opendex-trades",
            "opendex-states",
        ];

        for topic_str in &topics {
            let topic = IdentTopic::new(topic_str.to_string());
            if let Err(e) = self.swarm.behaviour_mut().gossipsub.subscribe(&topic) {
                tracing::warn!("Failed to subscribe to {}: {}", topic_str, e);
            } else {
                tracing::info!("Subscribed to topic: {}", topic_str);
            }
        }
    }

    /// Observe network traffic for a given duration
    async fn observe(&mut self, duration_secs: u64) {
        tracing::info!("Observing network for {} seconds...", duration_secs);
        let start = Instant::now();

        while start.elapsed().as_secs() < duration_secs {
            match tokio::time::timeout(
                std::time::Duration::from_millis(100),
                self.swarm.next(),
            )
            .await
            {
                Ok(Some(event)) => self.handle_swarm_event(event),
                Ok(None) => break,
                Err(_) => {
                    // Timeout, continue observing
                }
            }
        }

        tracing::info!(
            "Observation complete. Metrics: {:?}",
            self.metrics
        );
    }

    /// Handle swarm events
    fn handle_swarm_event(&mut self, event: libp2p::swarm::SwarmEvent<ObserverBehaviour>) {
        match event {
            libp2p::swarm::SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                tracing::info!("Connected to peer: {}", peer_id);
                self.connected_peers.insert(
                    peer_id,
                    Multiaddr::empty().with(Protocol::P2p(peer_id)),
                );
            }
            libp2p::swarm::SwarmEvent::ConnectionClosed { peer_id, .. } => {
                tracing::info!("Disconnected from peer: {}", peer_id);
                self.connected_peers.remove(&peer_id);
            }
            libp2p::swarm::SwarmEvent::Behaviour(ObserverBehaviourEvent::Gossipsub(
                gossipsub::Event::Message { message, .. },
            )) => {
                self.metrics.messages_received += 1;
                self.handle_gossipsub_message(&message);
            }
            _ => {}
        }
    }

    /// Handle incoming GossipSub messages
    fn handle_gossipsub_message(&mut self, message: &GossipSubMessage) {
        // Try to parse the message to check for validity
        match std::str::from_utf8(&message.data) {
            Ok(json_str) => {
                match serde_json::from_str::<serde_json::Value>(json_str) {
                    Ok(_value) => {
                        self.metrics.messages_accepted += 1;
                    }
                    Err(_e) => {
                        // Malformed message detected
                        self.metrics.messages_rejected += 1;
                        tracing::debug!("Malformed message detected and rejected");
                    }
                }
            }
            Err(_e) => {
                // Invalid UTF-8
                self.metrics.messages_rejected += 1;
                tracing::debug!("Invalid UTF-8 message detected and rejected");
            }
        }
    }

    /// Execute a single defense validation
    async fn execute_validation(
        &mut self,
        scenario_id: &str,
        category: &str,
        description: &str,
        severity: &str,
        validate_fn: impl FnOnce(&mut BlueTeamAgent) -> tokio::task::JoinHandle<Result<(ValidationStatus, String)>>>,
    ) -> ValidationResult {
        let start = Instant::now();

        tracing::info!("Validating defense: {} - {}", scenario_id, description);

        // Capture baseline metrics
        let baseline = self.metrics.clone();

        let handle = validate_fn(self);
        let result = handle.await;

        let duration_ms = start.elapsed().as_millis() as u64;

        let (status, details) = match result {
            Ok(Ok((status, details))) => (status, details),
            Ok(Err(e)) => {
                tracing::warn!("Validation {} error: {}", scenario_id, e);
                (ValidationStatus::Error, e.to_string())
            }
            Err(e) => {
                tracing::error!("Validation {} panic: {}", scenario_id, e);
                (ValidationStatus::Error, e.to_string())
            }
        };

        // Calculate metric deltas
        let mut metrics_collected = HashMap::new();
        metrics_collected.insert(
            "messages_received_delta".to_string(),
            (self.metrics.messages_received - baseline.messages_received).to_string(),
        );
        metrics_collected.insert(
            "messages_rejected_delta".to_string(),
            (self.metrics.messages_rejected - baseline.messages_rejected).to_string(),
        );
        metrics_collected.insert(
            "messages_accepted_delta".to_string(),
            (self.metrics.messages_accepted - baseline.messages_accepted).to_string(),
        );

        let result = ValidationResult {
            scenario_id: scenario_id.to_string(),
            category: category.to_string(),
            description: description.to_string(),
            status,
            severity: severity.to_string(),
            duration_ms,
            details,
            metrics_collected,
        };

        self.results.push(result.clone());
        result
    }

    /// Run all protocol defense validations
    async fn run_protocol_defenses(&mut self, iterations: usize) {
        for i in 0..iterations {
            tracing::info!("Protocol defense iteration {}/{}", i + 1, iterations);

            // BD-001: Message parsing robustness
            self.execute_validation(
                "BD-001",
                "Protocol",
                "Message parsing robustness",
                "Medium",
                |agent| protocol_defenses::validate_message_parsing_robustness(agent),
            ).await;

            // BD-002: Replay protection
            self.execute_validation(
                "BD-002",
                "Protocol",
                "Replay protection",
                "High",
                |agent| protocol_defenses::validate_replay_protection(agent),
            ).await;

            // BD-003: Sequence validation
            self.execute_validation(
                "BD-003",
                "Protocol",
                "Sequence validation",
                "Medium",
                |agent| protocol_defenses::validate_sequence_validation(agent),
            ).await;

            // BD-004: Topic isolation
            self.execute_validation(
                "BD-004",
                "Protocol",
                "Topic isolation",
                "Low",
                |agent| protocol_defenses::validate_topic_isolation(agent),
            ).await;

            // BD-005: Peer authentication
            self.execute_validation(
                "BD-005",
                "Protocol",
                "Peer authentication",
                "High",
                |agent| protocol_defenses::validate_peer_authentication(agent),
            ).await;

            // BD-006: Message rate limiting
            self.execute_validation(
                "BD-006",
                "Protocol",
                "Message rate limiting",
                "High",
                |agent| protocol_defenses::validate_rate_limiting(agent),
            ).await;

            // BD-007: DHT integrity
            self.execute_validation(
                "BD-007",
                "Protocol",
                "DHT integrity",
                "Low",
                |agent| protocol_defenses::validate_dht_integrity(agent),
            ).await;

            // BD-008: Connection limits
            self.execute_validation(
                "BD-008",
                "Protocol",
                "Connection limits",
                "High",
                |agent| protocol_defenses::validate_connection_limits(agent),
            ).await;
        }
    }

    /// Run all crypto defense validations
    async fn run_crypto_defenses(&mut self, iterations: usize) {
        for i in 0..iterations {
            tracing::info!("Crypto defense iteration {}/{}", i + 1, iterations);

            // BD-009: Token single-use
            self.execute_validation(
                "BD-009",
                "Crypto",
                "Token single-use",
                "High",
                |agent| crypto_defenses::validate_token_single_use(agent),
            ).await;

            // BD-010: Constant-time comparison
            self.execute_validation(
                "BD-010",
                "Crypto",
                "Constant-time comparison",
                "Medium",
                |agent| crypto_defenses::validate_constant_time_comparison(agent),
            ).await;

            // BD-011: Rate limiter activation
            self.execute_validation(
                "BD-011",
                "Crypto",
                "Rate limiter activation",
                "High",
                |agent| crypto_defenses::validate_rate_limiter_activation(agent),
            ).await;

            // BD-012: Signature strictness
            self.execute_validation(
                "BD-012",
                "Crypto",
                "Signature strictness",
                "High",
                |agent| crypto_defenses::validate_signature_strictness(agent),
            ).await;

            // BD-013: Key generation quality
            self.execute_validation(
                "BD-013",
                "Crypto",
                "Key generation quality",
                "Medium",
                |agent| crypto_defenses::validate_key_generation_quality(agent),
            ).await;

            // BD-014: Password hash strength
            self.execute_validation(
                "BD-014",
                "Crypto",
                "Password hash strength",
                "High",
                |agent| crypto_defenses::validate_password_hash_strength(agent),
            ).await;

            // BD-015: Proof verification
            self.execute_validation(
                "BD-015",
                "Crypto",
                "Proof verification",
                "Critical",
                |agent| crypto_defenses::validate_proof_verification(agent),
            ).await;
        }
    }

    /// Run all DoS defense validations
    async fn run_dos_defenses(&mut self, iterations: usize) {
        for i in 0..iterations {
            tracing::info!("DoS defense iteration {}/{}", i + 1, iterations);

            // BD-020: State limits
            self.execute_validation(
                "BD-020",
                "DoS",
                "State limits",
                "Medium",
                |agent| dos_defenses::validate_state_limits(agent),
            ).await;

            // BD-021: Memory limits
            self.execute_validation(
                "BD-021",
                "DoS",
                "Memory limits",
                "High",
                |agent| dos_defenses::validate_memory_limits(agent),
            ).await;

            // BD-022: Connection timeouts
            self.execute_validation(
                "BD-022",
                "DoS",
                "Connection timeouts",
                "Medium",
                |agent| dos_defenses::validate_connection_timeouts(agent),
            ).await;
        }
    }

    /// Run all fiat defense validations
    async fn run_fiat_defenses(&mut self, iterations: usize) {
        for i in 0..iterations {
            tracing::info!("Fiat defense iteration {}/{}", i + 1, iterations);

            // BD-023: Payment verification
            self.execute_validation(
                "BD-023",
                "Fiat",
                "Payment verification",
                "High",
                |agent| fiat_defenses::validate_payment_verification(agent),
            ).await;

            // BD-024: Reversal detection
            self.execute_validation(
                "BD-024",
                "Fiat",
                "Reversal detection",
                "High",
                |agent| fiat_defenses::validate_reversal_detection(agent),
            ).await;

            // BD-025: Atomic transitions
            self.execute_validation(
                "BD-025",
                "Fiat",
                "Atomic transitions",
                "Critical",
                |agent| fiat_defenses::validate_atomic_transitions(agent),
            ).await;

            // BD-026: Chargeback monitoring
            self.execute_validation(
                "BD-026",
                "Fiat",
                "Chargeback monitoring",
                "High",
                |agent| fiat_defenses::validate_chargeback_monitoring(agent),
            ).await;

            // BD-027: Multisig enforcement
            self.execute_validation(
                "BD-027",
                "Fiat",
                "Multisig enforcement",
                "Critical",
                |agent| fiat_defenses::validate_multisig_enforcement(agent),
            ).await;

            // BD-028: State machine integrity
            self.execute_validation(
                "BD-028",
                "Fiat",
                "State machine integrity",
                "High",
                |agent| fiat_defenses::validate_state_machine_integrity(agent),
            ).await;

            // BD-029: Currency validation
            self.execute_validation(
                "BD-029",
                "Fiat",
                "Currency validation",
                "Medium",
                |agent| fiat_defenses::validate_currency_validation(agent),
            ).await;
        }
    }

    /// Run all transport defense validations
    async fn run_transport_defenses(&mut self, iterations: usize) {
        for i in 0..iterations {
            tracing::info!("Transport defense iteration {}/{}", i + 1, iterations);

            // BD-030: Envelope signature verification
            self.execute_validation(
                "BD-030",
                "Transport",
                "Envelope signature verification",
                "High",
                |agent| transport_defenses::validate_envelope_signature(agent),
            ).await;

            // BD-031: Message timestamp validation
            self.execute_validation(
                "BD-031",
                "Transport",
                "Message timestamp validation",
                "Medium",
                |agent| transport_defenses::validate_timestamp_validation(agent),
            ).await;

            // BD-032: Transport message size limits
            self.execute_validation(
                "BD-032",
                "Transport",
                "Transport message size limits",
                "Medium",
                |agent| transport_defenses::validate_message_size_limits(agent),
            ).await;
        }
    }

    /// Print summary of all validation results
    fn print_summary(&self) {
        println!("\n========================================");
        println!("Blue Team Defense Validation Summary");
        println!("========================================\n");

        let total = self.results.len();
        let passed = self.results.iter().filter(|r| r.status == ValidationStatus::Pass).count();
        let failed = self.results.iter().filter(|r| r.status == ValidationStatus::Fail).count();
        let partial = self.results.iter().filter(|r| r.status == ValidationStatus::Partial).count();
        let errors = self.results.iter().filter(|r| r.status == ValidationStatus::Error).count();

        println!("Total validations: {}", total);
        println!("Passed: {}", passed);
        println!("Failed: {}", failed);
        println!("Partial: {}", partial);
        println!("Errors: {}", errors);

        if total > 0 {
            let detection_rate = (passed as f64 / total as f64) * 100.0;
            println!("\nDetection rate: {:.1}%", detection_rate);
        }
        println!();

        // Group by category
        let mut categories: HashMap<String, Vec<&ValidationResult>> = HashMap::new();
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

        // Print details for failed validations
        if failed > 0 || errors > 0 {
            println!("Failed/Error Validations:");
            for result in &self.results {
                if result.status == ValidationStatus::Fail || result.status == ValidationStatus::Error {
                    println!("  {} - {}", result.scenario_id, result.details);
                }
            }
        }

        // Print observation metrics
        println!("\nObservation Metrics:");
        println!("  Messages received: {}", self.metrics.messages_received);
        println!("  Messages accepted: {}", self.metrics.messages_accepted);
        println!("  Messages rejected: {}", self.metrics.messages_rejected);
        println!("  Replay detections: {}", self.metrics.replay_detections);
        println!("  Auth failures: {}", self.metrics.auth_failures);
        println!("  Rate limiter activations: {}", self.metrics.rate_limiter_activations);
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

    tracing::info!("Starting Blue Team Observer Agent");

    // Create agent
    let mut agent = BlueTeamAgent::new(args.port).await?;

    // Connect to peers if addresses provided
    if !args.connect.is_empty() {
        agent.connect_to_peers(&args.connect).await?;
    }

    // Subscribe to all topics
    agent.subscribe_all_topics();

    // Execute validations based on arguments
    if args.all {
        tracing::info!("Validating all defenses");
        agent.run_protocol_defenses(args.iterations).await;
        agent.run_crypto_defenses(args.iterations).await;
        agent.run_dos_defenses(args.iterations).await;
        agent.run_fiat_defenses(args.iterations).await;
        agent.run_transport_defenses(args.iterations).await;
    } else if let Some(ref category) = args.category {
        tracing::info!("Validating category: {}", category);
        match category.to_lowercase().as_str() {
            "protocol" => agent.run_protocol_defenses(args.iterations).await,
            "crypto" => agent.run_crypto_defenses(args.iterations).await,
            "dos" => agent.run_dos_defenses(args.iterations).await,
            "fiat" => agent.run_fiat_defenses(args.iterations).await,
            "transport" => agent.run_transport_defenses(args.iterations).await,
            _ => {
                eprintln!("Unknown category: {}. Valid categories: protocol, crypto, dos, fiat, transport", category);
                std::process::exit(1);
            }
        }
    } else if let Some(ref scenario_id) = args.scenario {
        tracing::info!("Validating scenario: {}", scenario_id);
        // Dispatch to specific scenario based on ID prefix
        if scenario_id.starts_with("BD-00") || scenario_id.starts_with("BD-01") {
            agent.run_protocol_defenses(1).await;
        } else if scenario_id.starts_with("BD-02") {
            if scenario_id <= "BD-022" {
                agent.run_dos_defenses(1).await;
            } else {
                agent.run_fiat_defenses(1).await;
            }
        } else if scenario_id.starts_with("BD-03") {
            agent.run_transport_defenses(1).await;
        } else {
            eprintln!("Unknown scenario ID: {}. Use BD- prefix.", scenario_id);
            std::process::exit(1);
        }
    } else {
        // Default: passive observation mode
        tracing::info!("Running in passive observation mode for {} seconds", args.observe_duration);
        agent.observe(args.observe_duration).await;
    }

    // Print summary
    agent.print_summary();

    Ok(())
}