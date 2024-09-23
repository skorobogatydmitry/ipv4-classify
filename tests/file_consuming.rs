use ipv4_classify;

#[test]
#[should_panic(
    expected = "called `Result::unwrap()` on an `Err` value: \"unable to parse \\\"256.0.1.2\\\": ParseIntError { kind: PosOverflow }\""
)]
fn file_has_wrong_addr() {
    ipv4_classify::find_subnets(vec!["tests/res/invalid_ips.csv".to_string()]).unwrap();
}

#[test]
#[should_panic(
    expected = "called `Result::unwrap()` on an `Err` value: Os { code: 2, kind: NotFound, message: \"No such file or directory\" }"
)]
fn file_does_not_exist() {
    ipv4_classify::find_subnets(vec!["non.file".to_string()]).unwrap();
}
