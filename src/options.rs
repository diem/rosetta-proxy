use structopt::StructOpt;
use url::Url;

#[derive(Clone, Debug, StructOpt)]
pub struct Options {
    #[structopt(long, parse(try_from_str = Url::parse))]
    pub diem_endpoint: Url,

    #[structopt(long)]
    pub network: String,
}
