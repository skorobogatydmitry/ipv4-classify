use crate::*;
use std::{net::Ipv4Addr, ops::Sub, str::FromStr};

#[test]
fn subnet_root() {
    let root = Subnet::root();
    assert_eq!(0, root.bits);
    assert_eq!(0, root.mask_len);
    assert_eq!(0, root.mask);
}

#[test]
fn subnet_new() {
    let s = Subnet::new(1, 2, 3, 4, 32).unwrap();
    assert_eq!(0x01_02_03_04, s.bits);
    assert_eq!(0xFF_FF_FF_FF, s.mask);
    assert_eq!(32, s.mask_len);

    let s = Subnet::new(10, 2, 3, 4, 24).unwrap();
    assert_eq!(0x0A_02_03_00, s.bits);
    assert_eq!(0xFF_FF_FF_00, s.mask);
    assert_eq!(24, s.mask_len);
}

#[test]
#[should_panic]
fn subnet_new_too_long_mask() {
    Subnet::new(1, 2, 3, 4, 35).unwrap();
}

#[test]
fn subnet_from_str() {
    let s = Subnet::from_str("1.2.3.4/24").unwrap();
    assert_eq!(24, s.mask_len);
    assert_eq!(0xFF_FF_FF_00, s.mask);
    assert_eq!(0x01_02_03_00, s.bits);
}

#[test]
#[should_panic]
fn subnet_from_str_wrong_mask() {
    let s = Subnet::from_str("1.2.3.4/35").unwrap();
}

#[test]
fn subnet_from_str_ip() {
    let s = Subnet::from_str("1.2.3.7").unwrap();
    assert_eq!(32, s.mask_len);
    assert_eq!(0xFF_FF_FF_FF, s.mask);
    assert_eq!(0x01_02_03_07, s.bits);
}

#[test]
fn subnet_from_str_too_many_slash() {
    assert_eq!(
        "there are more than 1 / in the address",
        Subnet::from_str("1/2.3/7").err().unwrap().to_string()
    );
}

#[test]
fn subnet_from_str_too_big_mask() {
    assert_eq!(
        "can't parse netmask from 1.2.3.7/300",
        Subnet::from_str("1.2.3.7/300").err().unwrap().to_string()
    );
}

#[test]
fn subnet_from_str_wrong_octets_cnt() {
    assert_eq!(
        "address 1.2.3.7.8 doesn't have 4 dot-separated octets",
        Subnet::from_str("1.2.3.7.8").err().unwrap().to_string()
    );
}

#[test]
fn subnet_from_str_too_big_octet() {
    assert_eq!(
        "unable to parse \"1.2.3.257\": ParseIntError { kind: PosOverflow }",
        Subnet::from_str("1.2.3.257").err().unwrap().to_string()
    );
}

#[test]
fn subnet_common_of_2_addrs() {
    let s1 = Subnet::new(10, 1, 2, 3, 32).unwrap();
    let s2 = Subnet::new(10, 1, 2, 4, 32).unwrap();
    let result = Subnet::new(10, 1, 2, 0, 29).unwrap();
    assert_eq!(result, Subnet::common_of(&s1, &s2, None).unwrap());
}

#[test]
fn subnet_common_of_2_subnets() {
    let s1 = Subnet::new(10, 1, 2, 255, 24).unwrap();
    let s2 = Subnet::new(10, 1, 2, 240, 26).unwrap();
    let result = Subnet::new(10, 1, 2, 0, 24).unwrap();
    assert_eq!(result, Subnet::common_of(&s1, &s2, None).unwrap());
}

#[test]
fn subnet_common_of_2_subnets_extending_prefix() {
    let s1 = Subnet::new(10, 128, 0, 0, 24).unwrap();
    let s2 = Subnet::new(10, 0, 2, 0, 24).unwrap();
    let result = Subnet::new(10, 0, 0, 0, 8).unwrap();
    assert_eq!(result, Subnet::common_of(&s1, &s2, None).unwrap());
}

#[test]
fn subnet_common_of_2_subnets_extending_subnet_outside_limit() {
    let s1 = Subnet::new(10, 128, 0, 0, 24).unwrap();
    let s2 = Subnet::new(10, 0, 2, 0, 24).unwrap();
    assert_eq!(None, Subnet::common_of(&s1, &s2, Some(16)));
}
