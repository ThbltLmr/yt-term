use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub struct Args {
    #[clap(
        short,
        long,
        default_value = "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
    )]
    pub url: String,
}

pub fn parse_args() -> Args {
    Args::parse()
}
