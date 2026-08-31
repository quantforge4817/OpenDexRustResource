use p2p_node::cli::{CliAction, parse_args};

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn defaults_are_correct() {
    let action = parse_args(&[]).unwrap();

    assert_eq!(
        action,
        CliAction::Run(p2p_node::cli::CliArgs {
            port: 4001,
            peer: None,
            new_identity: false,
        })
    );
}

#[test]
fn parses_port() {
    let action = parse_args(&args(&["--port", "5000"])).unwrap();

    assert_eq!(
        action,
        CliAction::Run(p2p_node::cli::CliArgs {
            port: 5000,
            peer: None,
            new_identity: false,
        })
    );
}

#[test]
fn parses_peer() {
    let action = parse_args(&args(&["--peer", "/ip4/127.0.0.1/tcp/4001/p2p/test"])).unwrap();

    match action {
        CliAction::Run(config) => {
            assert_eq!(config.port, 4001);
            assert_eq!(
                config.peer.as_deref(),
                Some("/ip4/127.0.0.1/tcp/4001/p2p/test")
            );
            assert!(!config.new_identity);
        }
        CliAction::Help => panic!("expected Run"),
    }
}

#[test]
fn parses_new_identity() {
    let action = parse_args(&args(&["--new-identity"])).unwrap();

    match action {
        CliAction::Run(config) => {
            assert!(config.new_identity);
        }
        CliAction::Help => panic!("expected Run"),
    }
}

#[test]
fn parses_all_options() {
    let action = parse_args(&args(&[
        "--port",
        "5000",
        "--peer",
        "/ip4/127.0.0.1/tcp/4001/p2p/test",
        "--new-identity",
    ]))
    .unwrap();

    assert_eq!(
        action,
        CliAction::Run(p2p_node::cli::CliArgs {
            port: 5000,
            peer: Some("/ip4/127.0.0.1/tcp/4001/p2p/test".to_string()),
            new_identity: true,
        })
    );
}

#[test]
fn help_is_an_action() {
    assert_eq!(parse_args(&args(&["--help"])).unwrap(), CliAction::Help);

    assert_eq!(parse_args(&args(&["-h"])).unwrap(), CliAction::Help);
}

#[test]
fn rejects_missing_port() {
    assert!(parse_args(&args(&["--port"])).is_err());
}

#[test]
fn rejects_invalid_port() {
    assert!(parse_args(&args(&["--port", "abc"])).is_err());
}

#[test]
fn rejects_zero_port() {
    assert!(parse_args(&args(&["--port", "0"])).is_err());
}

#[test]
fn rejects_missing_peer() {
    assert!(parse_args(&args(&["--peer"])).is_err());
}

#[test]
fn rejects_unknown_argument() {
    assert!(parse_args(&args(&["--banana"])).is_err());
}
