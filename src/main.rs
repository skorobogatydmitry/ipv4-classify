use std::error::Error;

use argparse::{ArgumentParser, List, StoreTrue};
use ipv4_classify::{Config, IPINFO_TOKEN_FILE};

fn main() -> Result<(), Box<dyn Error>> {
    let mut file_names = vec![];
    let mut query_ipinfo = false;
    {
        let query_help = format!("Whether to use or not ipinfo.io to grab additional data about subnets. The tool expects API token to be at ~/{}", IPINFO_TOKEN_FILE);
        let mut arg_parser = ArgumentParser::new();
        arg_parser.set_description(
            "Split a long list of IPv4 addresses into subnets and verify them using ipinfo.io",
        );
        arg_parser.refer(&mut file_names).add_option(
            &["-f", "--files"],
            List,
            "List of files with ipv4 addresses to read e.g. -f one.txt another.txt",
        );
        arg_parser.refer(&mut query_ipinfo).add_option(
            &["-q", "--query-ipinfo"],
            StoreTrue,
            &query_help,
        );
        arg_parser.parse_args_or_exit();
    }
    let config = Config::new(file_names, query_ipinfo)?;
    if config.has_files() {
        let subnets = ipv4_classify::find_subnets(config.file_names)?;
        // ipv4_classify::recheck_subnets(config, subnets);
        Ok(())
    } else {
        Err("no files provided, try -h".into())
    }
}
