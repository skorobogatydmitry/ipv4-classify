use std::{env::args, path::Path};

fn main() {
    let args: Vec<String> = args().collect();
    let file_name = args
        .get(1)
        .expect("file name with list of addresses is required")
        .to_owned();
    if !Path::new(&file_name).exists() {
        panic!("file {} doesn't exist", file_name);
    }
    ipv4_classify::parse_file_to_tree(file_name)
}
