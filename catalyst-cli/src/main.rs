mod config;

use anyhow::Result;
use catalyst_core::Agent;
use catalyst_llm::{create_provider, Provider};
use catalyst_tools::ToolRegistry;
use catalyst_tui::{run_app, App};
use clap::Parser;
use config::Config;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

#[derive(Parser)]
#[command(name = "catalyst")]
#[command(about = "A research-driven AI coding agent", long_about = None)]
struct Cli {
    /// Working directory
    #[arg(short, long)]
    dir: Option<PathBuf>,

    /// Model to use
    #[arg(short, long)]
    model: Option<String>,

    /// Provider to use (anthropic or openrouter)
    #[arg(short, long)]
    provider: Option<String>,

    /// API key
    #[arg(long, env = "ANTHROPIC_API_KEY")]
    api_key: Option<String>,

    /// OpenRouter API key
    #[arg(long, env = "OPENROUTER_API_KEY")]
    openrouter_api_key: Option<String>,
}

struct SharedState {
    api_keys: HashMap<String, String>,
    provider: String,
    model: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    let mut config = Config::load().unwrap_or_default();
    config.merge_cli_args(
        cli.model.clone(),
        cli.api_key.clone().or(cli.openrouter_api_key.clone()),
        cli.dir.clone(),
    );

    let provider_str = cli.provider.clone().unwrap_or(
        config
            .provider
            .clone()
            .unwrap_or_else(|| "anthropic".to_string()),
    );
    let provider_type = Provider::from_string(&provider_str).unwrap_or(Provider::Anthropic);

    let api_key = match provider_type {
        Provider::Anthropic => cli.api_key.clone().or(config.api_key.clone()),
        Provider::OpenRouter => cli.openrouter_api_key.clone().or(config.api_key.clone()),
    };

    let model = cli.model.unwrap_or_else(|| config.model.clone());

    let working_dir = cli
        .dir
        .or(config.working_dir.clone())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from("."));

    let mut api_keys = HashMap::new();
    if let Some(key) = api_key {
        api_keys.insert(provider_str.clone(), key);
    }

    let state = Arc::new(Mutex::new(SharedState {
        api_keys,
        provider: provider_str.clone(),
        model: model.clone(),
    }));

    let mut app = App::new(model.clone()).with_provider(provider_str.clone());

    if let Some(key) = config.api_key {
        app = app.with_api_key(&provider_str, key);
    } else if let Some(key) = cli.api_key.or(cli.openrouter_api_key) {
        app = app.with_api_key(&provider_str, key);
    }

    app.add_system_message(
        format!("Provider: {} | Model: {}", provider_str, model),
        catalyst_tui::app::SystemLevel::Info,
    );

    let has_key = app.get_api_key(&provider_str).is_some();
    if !has_key {
        app.add_system_message(
            "No API key configured. Use /model to select a provider and enter your API key."
                .to_string(),
            catalyst_tui::app::SystemLevel::Warning,
        );
    }

    app.add_system_message(
        "Welcome to Catalyst! Type /help for commands.".to_string(),
        catalyst_tui::app::SystemLevel::Info,
    );

    let (agent_tx, agent_rx) = mpsc::unbounded_channel();
    let (input_tx, mut input_rx) = mpsc::unbounded_channel::<String>();
    let (api_key_tx, mut api_key_rx) = mpsc::unbounded_channel::<(String, String)>();

    let agent: Arc<Mutex<Option<Agent>>> = Arc::new(Mutex::new(None));
    let state_clone = state.clone();
    let agent_clone = agent.clone();
    let agent_tx_clone = agent_tx.clone();
    let working_dir_arc = Arc::new(working_dir);

    let state_for_api = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(user_input) = input_rx.recv() => {
                    let state_guard = state_clone.lock().await;
                    let provider_str = state_guard.provider.clone();
                    let model = state_guard.model.clone();
                    let api_key = state_guard.api_keys.get(&provider_str).cloned();
                    drop(state_guard);

                    if let Some(key) = api_key {
                        let mut agent_guard = agent_clone.lock().await;

                        if agent_guard.is_none() {
                            let provider_type = Provider::from_string(&provider_str).unwrap_or(Provider::Anthropic);
                            let provider = create_provider(provider_type, key.clone(), model.clone());
                            let tools = ToolRegistry::new();
                            *agent_guard = Some(Agent::new(provider, tools, (*working_dir_arc).clone()));
                        }

                        if let Some(ref mut agent_instance) = *agent_guard {
                            let _ = agent_instance.send(user_input, agent_tx_clone.clone()).await;
                        }
                    } else {
                        let _ = agent_tx_clone.send(catalyst_core::AgentEvent::Error(
                            "No API key configured. Use /model to set your API key.".to_string()
                        ));
                    }
                }
                Some((provider, api_key)) = api_key_rx.recv() => {
                    let mut state_guard = state_for_api.lock().await;
                    state_guard.api_keys.insert(provider.clone(), api_key);
                    state_guard.provider = provider;
                    let mut agent_guard = agent_clone.lock().await;
                    *agent_guard = None;
                }
                else => break,
            }
        }
    });

    run_app(&mut app, agent_rx, input_tx, api_key_tx).await?;

    Ok(())
}
