use std::{
    cmp,
    collections::HashMap,
    fmt::{Debug, Display, Formatter},
    fs,
    mem::replace,
    net::Ipv4Addr,
    str::FromStr,
};

#[cfg(test)]
mod test;

pub fn parse_file_to_tree(file_name: String) {
    let mut addrs: Vec<Ipv4Addr> = fs::read_to_string(&file_name)
        .expect(&format!("can't read {}", file_name))
        .split("\n")
        .map(|el| el.trim())
        .filter(|el| !el.is_empty())
        .map(|str_addr| {
            Ipv4Addr::from_str(str_addr)
                .expect(&format!("cannot parse {} as IPv4 address", str_addr))
        })
        .collect();
    eprintln!("there are {} addresses", addrs.len());
    if let Some(seed) = addrs.pop() {
        let mut address_tree = AddressTree::new(&seed);
        for addr in addrs {
            match address_tree.push(addr) {
                Ok(_) => eprintln!("address {} fits the prefix {}", addr, address_tree.prefix),
                Err(addr) => {
                    let curr_prefix = &address_tree.prefix;
                    let addr_prefix = Prefix::from_addr(&addr);
                    if curr_prefix.bean == addr_prefix.bean & curr_prefix.mask {
                        panic!(
                            "address {} belongs to the current prefix {}, but wasn't consumed!",
                            addr, curr_prefix
                        )
                    } else {
                        // in all other case we need to build a new tree
                        match Prefix::common_of(curr_prefix, &addr_prefix, None) {
                            Some(new_prefix) => {
                                address_tree.stepdown(new_prefix, AddressTree::new(&addr))
                            }
                            None => panic!(
                                "no common prefix found for prefixes {} and {}!",
                                curr_prefix, addr_prefix
                            ),
                        }
                    }
                }
            }
        }
        println!("all subnets are:");
        for (subnet, ips) in address_tree.get_subnets_map() {
            println!("{} subnet", subnet);
            println!("\t{}", ips.join("\n\t"));
        }
    } else {
        panic!("addresses list is empty");
    }
}

#[derive(Debug, PartialEq)]
struct Prefix {
    bean: u32,    // IP address with significant bits representing the subnet
    mask_len: u8, // number of significant bits in the bean
    mask: u32,    // prebuilt number with leading significant bits set
}

impl Prefix {
    /// checks whether the prefix includes the address
    pub fn contains(&self, addr: &Ipv4Addr) -> bool {
        let addr_number = u32::from_be_bytes(addr.octets());
        return addr_number & self.mask == self.bean;
    }

    #[cfg(test)]
    pub fn set_mask(&mut self, new_mask_len: u8) {
        self.mask_len = new_mask_len;
        self.mask = u32::MAX << (32 - new_mask_len);
    }

    // make prefix from the address
    pub fn from_addr(addr: &Ipv4Addr) -> Self {
        Self {
            bean: u32::from_be_bytes(addr.octets()),
            mask_len: 32,
            mask: u32::MAX,
        }
    }

    /// find and return the closest common of the two prefixes if exists
    /// min_mask defines minimal (shortest) mask to look for
    /// e.g. 10.0.0.0/24 and 10.128.0.0/24 are both of 10.0.0.0/8
    /// if min_mask is 16 returns None for the above ranges,
    /// as 8 is less than min_mask - it's the only case when None can be returned,
    /// as default values for min_mask is 0, so 0.0.0.0/0 is the worst case
    /// # Panics
    /// if min_mask is bigger than any of the prefix masks
    pub fn common_of(p1: &Prefix, p2: &Prefix, min_mask: Option<u8>) -> Option<Prefix> {
        let min_mask = match min_mask {
            Some(min_mask) => min_mask,
            None => 0,
        };
        // get the shortest prefix to start from
        let mut curr_mask_len = cmp::min(p1.mask_len, p2.mask_len);
        if min_mask > curr_mask_len {
            panic!("min_mask {} is bigger than {}", min_mask, curr_mask_len);
        }
        let mut curr_mask = u32::MAX << (32 - curr_mask_len);
        while curr_mask_len >= min_mask {
            if p1.bean & curr_mask == p2.bean & curr_mask {
                return Some(Prefix {
                    bean: p1.bean & curr_mask,
                    mask_len: curr_mask_len,
                    mask: curr_mask,
                });
            }
            curr_mask <<= 1;
            curr_mask_len -= 1;
        }
        None
    }
}

impl Display for Prefix {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_str(&format!(
            "{}.{}.{}.{}/{}",
            (self.bean & (0xFF << 24)) >> 24,
            (self.bean & (0xFF << 16)) >> 16,
            (self.bean & (0xFF << 8)) >> 8,
            self.bean & 0xFF,
            self.mask_len
        ))
    }
}

#[derive(Debug)]
struct AddressTree {
    prefix: Prefix,
    children: Option<Vec<AddressTree>>,
}

impl AddressTree {
    pub fn new(seed: &Ipv4Addr) -> Self {
        AddressTree {
            prefix: Prefix::from_addr(seed),
            children: None,
        }
    }

    /// try to place the supplied address in the tree
    /// # Returns
    /// Ok(()) - if address was adopted by the tree
    /// Err(new_addr) - if it doesn't belond to the subtree
    pub fn push(&mut self, new_addr: Ipv4Addr) -> Result<(), Ipv4Addr> {
        if self.prefix.contains(&new_addr) {
            if let Some(ref mut children) = self.children {
                let mut is_consumed = false;
                for ch in children.iter_mut() {
                    is_consumed = match ch.push(new_addr) {
                        Ok(_) => true, // address found its place, nothing to do here
                        Err(new_addr) => {
                            // it wasn't consumed - try to adopt
                            match Prefix::common_of(
                                &ch.prefix,
                                &Prefix::from_addr(&new_addr),
                                Some(self.prefix.mask_len + 1),
                            ) {
                                Some(new_prefix) => {
                                    ch.stepdown(new_prefix, AddressTree::new(&new_addr));
                                    true // found there's something in common
                                }
                                None => false, // the addr doesn't have anything in common with the child
                            }
                        }
                    };
                }
                if !is_consumed {
                    children.push(AddressTree::new(&new_addr));
                }
                return Ok(());
            } else {
                self.children = Some(vec![AddressTree {
                    prefix: Prefix::from_addr(&new_addr),
                    children: None,
                }]);
                Ok(())
            }
        } else {
            Err(new_addr)
        }
    }

    pub fn stepdown(&mut self, new_prefix: Prefix, neighbour: AddressTree) {
        let my_prefix = replace(&mut self.prefix, new_prefix);
        let new_me = match self.children.take() {
            Some(children) => AddressTree {
                prefix: my_prefix,
                children: Some(children),
            },
            None => AddressTree {
                prefix: my_prefix,
                children: None,
            },
        };

        self.children = Some(vec![new_me, neighbour]);
    }

    // return vector of "subnets" - prefixes that contain at least one tree leaf
    fn get_subnets(&self) -> Vec<&AddressTree> {
        let mut res = vec![];
        if let Some(ref children) = self.children {
            for ch in children {
                if ch.prefix.mask_len == 32 && ch.children.is_none() {
                    res.push(self);
                    break; // chop the subtree at the first IP address in it
                }
                res.append(&mut ch.get_subnets());
            }
        }
        res
    }

    fn get_leafs(&self) -> Vec<&AddressTree> {
        let mut res = vec![];
        if let Some(ref children) = self.children {
            for ch in children {
                if ch.children.is_none() {
                    res.push(ch);
                } else {
                    res.append(&mut ch.get_leafs());
                }
            }
        }
        res
    }

    /// make a human-readable map of subnets to all their addresses
    fn get_subnets_map(&self) -> HashMap<String, Vec<String>> {
        let subnets = self.get_subnets();
        let mut res = HashMap::new();

        for s in subnets {
            res.insert(
                s.prefix.to_string(),
                s.get_leafs()
                    .iter()
                    .map(|leaf| leaf.prefix.to_string())
                    .collect(),
            );
        }
        res
    }
}

impl Display for AddressTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_str(&format!("{}", self.prefix))?;
        if let Some(ref children) = self.children {
            f.write_str("=>[")?;
            for ref ch in children {
                <AddressTree as Display>::fmt(&ch, f)?;
            }
            f.write_str("]")?;
        }
        f.write_str(";")
    }
}
