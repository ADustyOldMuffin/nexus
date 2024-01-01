use clap::{Parser, command};
use local_ip_address::local_ip;
use serf::SerfMemberBuilder;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The UDP port to listen on for gossip
    #[arg(long)]
    gossip_port: u16,

    #[arg(long)]
    client_addr: String,

    /// Addresses of other member nodes in the Serf group
    #[arg(short, long)]
    members: Vec<String>,
}

#[tokio::main]
async fn main() {
    let my_ip = local_ip().unwrap();
    println!("local ip {:?}", my_ip);

    let args = Args::parse();

    let subscriber = tracing_subscriber::FmtSubscriber::builder().with_max_level(tracing::Level::DEBUG).finish();
    // use that subscriber to process traces emitted after this point
    tracing::subscriber::set_global_default(subscriber).expect("failed to set global subscriber");

    let mut serf_member_builder = SerfMemberBuilder::default();
    serf_member_builder.with_start_join(args.members);

    if args.gossip_port != 0 {
        serf_member_builder.with_udp_port(args.gossip_port);
    }

    let mut serf_member = serf_member_builder.build().expect("failed to build serf member");

    serf_member.start().await.expect("failed to start");
}
