use std::{
    cmp,
    collections::HashMap,
    env::home_dir,
    error::Error,
    fmt::{Debug, Display, Formatter},
    fs,
    mem::replace,
    num::ParseIntError,
    path::Path,
};

#[cfg(test)]
mod test;

/// relative path to auth token in the current user's home dir
pub const IPINFO_TOKEN_FILE: &str = ".ipinfo/token";

/// parsed tool's config
pub struct Config {
    pub file_names: Vec<String>,
    ipinfo_token: Option<String>,
    ipinfo_use_cache: bool,
}

impl Config {
    pub fn new(file_names: Vec<String>, query_ipinfo: bool) -> Result<Config, Box<dyn Error>> {
        let ipinfo_token = if query_ipinfo {
            let full_path = home_dir()
                .ok_or("unable to get user home directory")?
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
            ipinfo_use_cache: true, // TODO: make configurable
        })
    }

    fn url_of(&self, address: &Subnet) -> String {
        format!(
            "https://ipinfo.io/{}?token={}",
            address.to_string(),
            self.ipinfo_token
                .as_ref()
                .expect("no token available to construct URL")
        )
    }

    pub fn has_files(&self) -> bool {
        !self.file_names.is_empty()
    }
}

/// # parse a \n-separated list of IP addresses from the provided files into subnets
/// # returns
/// Err - if one of the files cannot be read, some line isn't a correct IP address or smth else went terribly wrong
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
            .map(|str_addr| Subnet::from_str(str_addr))
            .collect::<Result<Vec<Subnet>, Box<dyn Error>>>()?;

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

/// WIP
pub fn recheck_subnets(config: Config, subnets: HashMap<String, Vec<Subnet>>) {
    let client = reqwest::Client::new();
    for (subnet, addrs) in subnets.iter() {
        let data = client.get(config.url_of(addrs.first().unwrap()));
    }
    // make cache in ~/.ipinfo/
    // exclude private subnets
    // for each IP in the subnet
    // - check its actual subnet using API
    // - ...
    todo!()
}

/// IPv4 subnet representation
/// consists of u32 and netmask
#[derive(Debug, PartialEq)]
struct Subnet {
    bits: u32,    // IP address with significant bits representing the subnet
    mask_len: u8, // number of significant bits in the bits
    mask: u32,    // prebuilt number with leading significant bits set
}

impl Subnet {
    /// root of all ipv4 addresses
    pub fn root() -> Self {
        Self {
            bits: 0,
            mask_len: 0,
            mask: 0,
        }
    }

    /// make subnet from octets & mask length
    /// clear any bits set below the mask: e.g. 1.2.3.4/24 is acceptable but gets transformed to 1.2.3.0/24
    pub fn new(o1: u8, o2: u8, o3: u8, o4: u8, mask_len: u8) -> Result<Self, Box<dyn Error>> {
        if mask_len > 32 {
            Err("mask len is > 32".into())
        } else {
            let mask = u32::MAX << (32 - mask_len);
            Ok(Self {
                bits: u32::from_be_bytes([o1, o2, o3, o4]) & mask,
                mask_len: mask_len,
                mask,
            })
        }
    }

    /// parse string with netmask into a subnet
    pub fn from_str(src: &str) -> Result<Self, Box<dyn Error>> {
        let (addr, mask_len) = if src.contains("/") {
            let split: Vec<&str> = src.split('/').collect();
            if split.len() != 2 {
                return Err("there are more than 1 / in the address".into());
            }
            if let Ok(mask_len) = split.get(1).unwrap().parse::<u8>() {
                (*split.get(0).unwrap(), mask_len)
            } else {
                return Err(format!("can't parse netmask from {}", src).into());
            }
        } else {
            (src, 32)
        };
        match addr
            .split('.')
            .map(|el| el.parse::<u8>())
            .collect::<Result<Vec<u8>, ParseIntError>>()
        {
            Ok(octets) => {
                if octets.len() != 4 {
                    Err(format!("address {} doesn't have 4 dot-separated octets", addr).into())
                } else {
                    Self::new(octets[0], octets[1], octets[2], octets[3], mask_len)
                }
            }
            Err(e) => Err(format!("unable to parse {:?}: {:?}", addr, e).into()),
        }
    }

    /// check whether subnet includes other subnet
    pub fn contains(&self, other: &Subnet) -> bool {
        if self.mask_len > other.mask_len {
            return false;
        }
        // let addr_number = u32::from_be_bytes(addr.octets());
        return other.bits & self.mask == self.bits;
    }

    /// find and return the closest common of the two subnets if exists
    /// min_mask defines minimal (shortest) mask to look for
    /// e.g. 10.0.0.0/24 and 10.128.0.0/24 are both of 10.0.0.0/8
    /// if min_mask is 16 returns None for the above ranges,
    /// as 8 is less than min_mask - it's the only case when None can be returned,
    /// as default values for min_mask is 0, so 0.0.0.0/0 is the worst case
    /// # Panics
    /// if min_mask is bigger than any of the subnet masks
    pub fn common_of(s1: &Subnet, s2: &Subnet, min_mask: Option<u8>) -> Option<Subnet> {
        let min_mask = match min_mask {
            Some(min_mask) => min_mask,
            None => 0,
        };
        // get the shortest mask to start from
        let mut curr_mask_len = cmp::min(s1.mask_len, s2.mask_len);
        if min_mask > curr_mask_len {
            panic!("min_mask {} is bigger than {}", min_mask, curr_mask_len);
        }
        let mut curr_mask = u32::MAX << (32 - curr_mask_len);
        while curr_mask_len >= min_mask {
            if s1.bits & curr_mask == s2.bits & curr_mask {
                return Some(Subnet {
                    bits: s1.bits & curr_mask,
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

impl Display for Subnet {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_str(&format!(
            "{}.{}.{}.{}/{}",
            (self.bits & (0xFF << 24)) >> 24,
            (self.bits & (0xFF << 16)) >> 16,
            (self.bits & (0xFF << 8)) >> 8,
            self.bits & 0xFF,
            self.mask_len
        ))
    }
}

#[derive(Debug)]
struct AddressTree {
    subnet: Subnet,
    children: Option<Vec<AddressTree>>,
}

impl AddressTree {
    /// make a new empty tree starting from 0.0.0.0/0
    pub fn new() -> Self {
        Self {
            subnet: Subnet::root(),
            children: None,
        }
    }

    /// make a new empty tree starting at subnet
    fn of(subnet: Subnet) -> Self {
        Self {
            subnet,
            children: None,
        }
    }

    /// try to place the supplied subnet in the tree
    /// # Returns
    /// Ok(()) - address was adopted by the tree
    /// Err(new_subnet) - supplied subnet doesn't belond to the current tree
    pub fn push(&mut self, new_subnet: Subnet) -> Result<(), Subnet> {
        eprintln!("attempt to push {} to {}", new_subnet, self.subnet);
        if self.subnet.contains(&new_subnet) {
            if let Some(ref mut children) = self.children {
                let mut to_consume = Some(new_subnet);
                for ch in children.iter_mut() {
                    eprintln!("processing subnet {}", ch.subnet);
                    // check whether there's an address to take
                    if let Some(new_subnet) = to_consume.take() {
                        match ch.push(new_subnet) {
                            Ok(_) => return Ok(()), // address found its place, nothing to do here
                            Err(new_subnet) => {
                                // it wasn't consumed - try to adopt
                                match Subnet::common_of(
                                    &ch.subnet,
                                    &new_subnet,
                                    Some(self.subnet.mask_len + 1),
                                ) {
                                    Some(new_intermediate) => {
                                        eprintln!(
                                            "address {} and {} are joined into {}",
                                            new_subnet, ch.subnet, new_intermediate
                                        );
                                        ch.stepdown(new_intermediate, AddressTree::of(new_subnet));
                                    }
                                    None => to_consume = Some(new_subnet),
                                }
                            }
                        }
                    } else {
                        // address was placed
                        return Ok(());
                    }
                }
                if let Some(new_subnet) = to_consume.take() {
                    eprintln!("address {} settled in {}", new_subnet, self.subnet);
                    children.push(AddressTree::of(new_subnet));
                }
            } else {
                self.children = Some(vec![AddressTree::of(new_subnet)]);
            }
            Ok(())
        } else {
            Err(new_subnet)
        }
    }

    fn stepdown(&mut self, new_subnet: Subnet, neighbour: AddressTree) {
        let my_subnet = replace(&mut self.subnet, new_subnet);
        let new_me = match self.children.take() {
            Some(children) => AddressTree {
                subnet: my_subnet,
                children: Some(children),
            },
            None => AddressTree {
                subnet: my_subnet,
                children: None,
            },
        };

        self.children = Some(vec![new_me, neighbour]);
    }

    /// extract vector of "subnets" - subnets that contain at least one tree leaf (IP address)
    fn get_subnets(&self) -> Vec<&AddressTree> {
        let mut res = vec![];
        if let Some(ref children) = self.children {
            if children.iter().any(|ch| ch.subnet.mask_len == 32) {
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
                s.subnet.to_string(),
                s.get_leafs()
                    .iter()
                    .map(|leaf| leaf.subnet.to_string())
                    .collect(),
            );
        }
        res
    }
}

impl Display for AddressTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_str(&format!("{}", self.subnet))?;
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
