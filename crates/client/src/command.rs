use clap::Parser;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, default_value = "db")]
    pub state_prefix: String,
    #[arg(short, long, default_value_t = 50001)]
    pub rpc_server_port: u16,
}

