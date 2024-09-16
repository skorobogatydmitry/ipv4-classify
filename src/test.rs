use crate::*;
use std::{net::Ipv4Addr, str::FromStr};

#[test]
fn prefix_common_of_2_addrs() {
    let p1 = Prefix::from_addr(&Ipv4Addr::from_str("10.1.2.3").unwrap());
    let p2 = Prefix::from_addr(&Ipv4Addr::from_str("10.1.2.4").unwrap());
    let mut result = Prefix::from_addr(&Ipv4Addr::from_str("10.1.2.0").unwrap());
    result.set_mask(29);
    assert_eq!(result, Prefix::common_of(&p1, &p2, None).unwrap());
}

#[test]
fn prefix_common_of_2_subnets() {
    let mut p1 = Prefix::from_addr(&Ipv4Addr::from_str("10.1.2.3").unwrap());
    p1.set_mask(24);
    let mut p2 = Prefix::from_addr(&Ipv4Addr::from_str("10.1.2.4").unwrap());
    p2.set_mask(26);
    let mut result = Prefix::from_addr(&Ipv4Addr::from_str("10.1.2.0").unwrap());
    result.set_mask(24);
    assert_eq!(result, Prefix::common_of(&p1, &p2, None).unwrap());
}

#[test]
fn prefix_common_of_2_subnets_extending_prefix() {
    let mut p1 = Prefix::from_addr(&Ipv4Addr::from_str("10.128.0.0").unwrap());
    p1.set_mask(24);
    let mut p2 = Prefix::from_addr(&Ipv4Addr::from_str("10.0.2.0").unwrap());
    p2.set_mask(24);
    let mut result = Prefix::from_addr(&Ipv4Addr::from_str("10.0.0.0").unwrap());
    result.set_mask(8);
    assert_eq!(result, Prefix::common_of(&p1, &p2, None).unwrap());
}

#[test]
fn prefix_common_of_2_subnets_extending_prefix_outside_limit() {
    let mut p1 = Prefix::from_addr(&Ipv4Addr::from_str("10.128.0.0").unwrap());
    p1.set_mask(24);
    let mut p2 = Prefix::from_addr(&Ipv4Addr::from_str("10.0.2.0").unwrap());
    p2.set_mask(24);
    assert_eq!(None, Prefix::common_of(&p1, &p2, Some(16)));
}
