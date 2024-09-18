use std::{
    cmp,
    collections::HashMap,
    env::home_dir,
    error::Error,
    fmt::{Debug, Display, Formatter},
    fs,
    mem::replace,
    net::{AddrParseError, Ipv4Addr},
    path::Path,
    str::FromStr,
};

#[cfg(test)]
mod test;

pub const IPINFO_TOKEN_FILE: &str = ".ipinfo/token";

/// tool's options
pub struct Config {
    pub file_names: Vec<String>,
    ipinfo_token: Option<String>,
}

impl Config {
    pub fn new(file_names: Vec<String>, query_ipinfo: bool) -> Result<Config, Box<dyn Error>> {
        let ipinfo_token = if query_ipinfo {
            let full_path = home_dir()
                .ok_or("Unable to get user home directory")?
                .join(IPINFO_TOKEN_FILE);
            eprintln!(
                "reading ipinfo.io token from {}",
                full_path.to_str().unwrap()
            );
            Some(fs::read_to_string(full_path)?)
        } else {
            None
        };
        for f in &file_names {
            if !Path::new(f).exists() {
                return Err(format!("file {} doesn't exist", f).into());
            }
        }

        Ok(Config {
            file_names,
            ipinfo_token,
        })
    }
}

pub fn find_subnets(
    file_names: Vec<String>,
) -> Result<HashMap<String, Vec<String>>, Box<dyn Error>> {
    let mut address_tree = AddressTree::new();

    for file_name in file_names {
        eprintln!("loading file {}", file_name);
        let mut addrs = fs::read_to_string(&file_name)?
            .split("\n")
            .map(|el| el.trim())
            .filter(|el| !el.is_empty())
            .map(|str_addr| Ipv4Addr::from_str(str_addr))
            .collect::<Result<Vec<Ipv4Addr>, AddrParseError>>()?;

        eprintln!("there are {} addresses in {}", addrs.len(), file_name);
        while let Some(addr) = addrs.pop() {
            match address_tree.push(addr) {
                Ok(_) => (),
                Err(addr) => {
                    return Err(
                        format!("address {} doesn't belong to IPv4 address space", addr).into(),
                    )
                }
            }
        }
    }
    println!("subnets found:");
    let subnets = address_tree.get_subnets_map();
    for (subnet, ips) in &subnets {
        println!("{} subnet", subnet);
        println!("\t{}", ips.join("\n\t"));
    }
    Ok(subnets)
}

fn recheck_subnets(_subnets: HashMap<Prefix, Vec<Ipv4Addr>>) {
    // for each IP in the subnet
    // - check its actual subnet using API
    // - ...
    todo!()
}

#[derive(Debug, PartialEq)]
struct Prefix {
    bean: u32,    // IP address with significant bits representing the subnet
    mask_len: u8, // number of significant bits in the bean
    mask: u32,    // prebuilt number with leading significant bits set
}

impl Prefix {
    /// root of all ipv4 addresses
    pub fn root() -> Self {
        Prefix {
            bean: 0,
            mask_len: 0,
            mask: 0,
        }
    }

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
    /// make a new empty tree starting from 0.0.0.0/0
    pub fn new() -> Self {
        AddressTree {
            prefix: Prefix::root(),
            children: None,
        }
    }

    // make a new empty tree starting at addr
    fn of(addr: &Ipv4Addr) -> Self {
        AddressTree {
            prefix: Prefix::from_addr(addr),
            children: None,
        }
    }

    /// try to place the supplied address in the tree
    /// # Returns
    /// Ok(()) - if address was adopted by the tree
    /// Err(new_addr) - if it doesn't belond to the subtree
    pub fn push(&mut self, new_addr: Ipv4Addr) -> Result<(), Ipv4Addr> {
        eprintln!("attempt to push {} to {}", new_addr, self.prefix);
        if self.prefix.contains(&new_addr) {
            if let Some(ref mut children) = self.children {
                let mut is_consumed = false;
                for ch in children.iter_mut() {
                    eprintln!("processing prefix {}", ch.prefix);
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
                                    eprintln!(
                                        "address {} and {} are joined into {}",
                                        new_addr, ch.prefix, new_prefix
                                    );
                                    ch.stepdown(new_prefix, AddressTree::of(&new_addr));
                                    true // found something in common
                                }
                                None => false, // the addr doesn't have anything in common with the child
                            }
                        }
                    };
                    if is_consumed {
                        break;
                    }
                }
                if !is_consumed {
                    eprintln!("address {} settled in {}", new_addr, self.prefix);
                    children.push(AddressTree::of(&new_addr));
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
            if children.iter().any(|ch| ch.prefix.mask_len == 32) {
                // chop the subtree at the first IP address in it
                res.push(self);
            } else {
                for ch in children {
                    res.append(&mut ch.get_subnets());
                }
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
