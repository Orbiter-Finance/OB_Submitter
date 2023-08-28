use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "submitter")]
#[command(author = "YanOctavian", version = "0.1.0", about = "submitter's client", long_about = None)]
pub struct Args {
    #[arg(short = 'p', long, default_value_t = 50001, help = "rpc server's port")]
    pub rpc_port: u16,
    #[arg(short = 'd', long, default_value_t = String::from("db"), help = "state db's path")]
    pub db_path: String,
    #[arg(short, long, default_value_t = false, help = "debug mode")]
    pub debug: bool,
}
