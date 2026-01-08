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
    Args::parse()
}
