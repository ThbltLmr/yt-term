use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub struct Args {
    #[clap(short, long, group = "input")]
    pub url: Option<String>,
    
    #[clap(short, long, group = "input")]
    pub search: Option<String>,
}

pub fn parse_args() -> Args {
    let args = Args::parse();
    
    // Ensure exactly one of url or search is provided
    if args.url.is_none() && args.search.is_none() {
        eprintln!("Error: Either --url or --search must be provided");
        std::process::exit(1);
    }
    
    args
}
