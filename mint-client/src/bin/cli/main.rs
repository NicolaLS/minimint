use clap::{Parser, Subcommand};
use mint_client::jsonrpc::client::JsonRpc;
use mint_client::jsonrpc::errors::RpcError;
use mint_client::jsonrpc::json::*;

#[derive(Parser)]
#[clap(
    name = "MiniMint CLI",
    about = "CLI to use the MiniMint RPC-Client (clientd)"
)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
    /// Clients holdings (total, coins, pending)
    Info {
        /// Format JSON
        #[clap(takes_value = false, short = 'r')]
        raw: bool,
    },
    /// Clients pending coins
    Pending {
        /// Format JSON
        #[clap(takes_value = false, short = 'r')]
        raw: bool,
    },
    /// The spend subcommand allows to send tokens to another client. This will select the smallest possible set of the client's coins that represents a given amount.
    #[clap(arg_required_else_help = true)]
    Spend {
        /// The amount of coins to be spend in msat if not set to sat
        amount: u64,
        /// Format JSON
        #[clap(takes_value = false, short = 'r')]
        raw: bool,
    },
    /// Reissue coins to claim them and avoid double spends
    #[clap(arg_required_else_help = true)]
    Reissue {
        /// The base64 encoded coins
        coins: String,
        /// Format JSON
        #[clap(takes_value = false, short = 'r')]
        raw: bool,
        #[clap(takes_value = false, short = 'v')]
        validate: bool,
    },
    Events {
        #[clap(takes_value = false, short = 'r')]
        raw: bool,
    },
}
#[tokio::main]
async fn main() {
    let args = Cli::parse();
    let jsonrpc = JsonRpc::default(); //how will the cli normaly ask for the host ?

    match args.command {
        Commands::Info { raw } => {
            info(&jsonrpc, raw).await;
        }
        Commands::Pending { raw } => {
            todo!()
        }
        Commands::Spend { amount, raw } => {
            todo!()
        }
        Commands::Reissue {
            coins,
            raw: pretty,
            validate,
        } => {
            todo!()
        }
        Commands::Events { raw } => {
            todo!()
        }
    }
}

async fn info(jsonrpc: &JsonRpc, raw: bool) {
    let response = jsonrpc.get_info().await;
    cli_print(raw, response);
}

fn cli_print(raw: bool, response: Result<APIResponse, Option<RpcError>>) {
    if let Err(None) = response {
        eprintln!("there was a problem with the rpc");
    } else {
        if raw {
            println!("{}", serde_json::to_string(&response).unwrap());
        } else {
            println!("{}", serde_json::to_string_pretty(&response).unwrap());
        }
    }
}
