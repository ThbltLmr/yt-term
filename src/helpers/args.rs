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

    #[clap(long, default_value = "640")]
    pub width: usize,

    #[clap(long, default_value = "360")]
    pub height: usize,

    #[clap(long, default_value = "25")]
    pub fps: usize,
}

pub fn parse_args() -> Args {
    Args::parse()
}
