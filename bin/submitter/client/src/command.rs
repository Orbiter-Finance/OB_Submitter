use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "submitter")]
#[command(author = "YanOctavian", version = "0.1.0", about = "submitter's client", long_about = None)]
pub struct Args {
    #[arg(short = 'p', long, default_value_t = 50001, help = "rpc server's port")]
    pub rpc_port: u16,
}
