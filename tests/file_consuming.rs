use ipv4_classify;

#[test]
#[should_panic(expected = "cannot parse 256.0.1.2 as IPv4 address: AddrParseError(Ipv4)")]
fn file_has_wrong_addr() {
    ipv4_classify::parse_file_to_tree("tests/res/invalid_ips.csv".to_string())
}

#[test]
#[should_panic(
    expected = "can't read non.file: Os { code: 2, kind: NotFound, message: \"No such file or directory\" }"
)]
fn file_does_not_exist() {
    ipv4_classify::parse_file_to_tree("non.file".to_string())
}
