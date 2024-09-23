use std::error::Error;

use argparse::{ArgumentParser, List};
use ipv4_classify::Config;

fn main() -> Result<(), Box<dyn Error>> {
    let mut file_names = vec![];
    {
        let mut arg_parser = ArgumentParser::new();
        arg_parser.set_description("Sort out a long list of IPv4 addresses into subnets");
        arg_parser.refer(&mut file_names).add_option(
            &["-f", "--files"],
            List,
            "List of files with ipv4 addresses to read e.g. -f one.txt another.txt",
        );
        arg_parser.parse_args_or_exit();
    }
    let config = Config::new(file_names)?;
    if config.has_files() {
        ipv4_classify::find_subnets(config.file_names)?;
        Ok(())
    } else {
        Err("no files provided, try -h".into())
    }
}
