use structopt::StructOpt;
use url::Url;

#[derive(Clone, Debug, StructOpt)]
pub struct Options {
    #[structopt(long, parse(try_from_str = Url::parse))]
    pub libra_endpoint: Url,

    #[structopt(long)]
    pub network: String,
}
