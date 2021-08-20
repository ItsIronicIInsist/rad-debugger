use std::mem::size_of;


//Use: mem rx2 <addr>. Read, 2 bytes, hex format
//n bytes: 1,2,4,8
//rw: Read or Write. Enum?
//format: x (hex. Default), u (unsigned base-10), d(signed base-10)

//use a HashSet for no duplictes? Still want an easy way to ensure - zero to one formats, one of r/w, zero to one of n-bytes
#[derive(Debug)]
pub struct rw_formatting {
	pub rw: String,
	pub format: String,
	pub n_bytes: u8,
}

//might need a better work around for 'format', cause if its signed in some cases Ill need to cast into signed, if its unsigned
//i might need to cast into unsigned. Sounds like metaprogramming/effort
impl rw_formatting {
	//default is hex formatting
	//and num of bytes is size of usize - 4 bytes for 32 bit arch, 8 bytes for 64 bit arch
	pub fn New() -> Self {
		rw_formatting {
			rw: String::new(),
			format: String::new(),
			n_bytes: size_of::<usize>() as u8,
		}
	}

	pub fn trim_val(&self, val: u64) -> u64 {
		let mut mask : u64 = 0;
		for i in (0..self.n_bytes) {
			mask = mask | (0xff << (i*8));
		}
		val & mask
	}
}


pub fn format_rw(rw_str: &str) -> Option<rw_formatting> {
	let mut rw_fmt = rw_formatting::New();
	
	let legal = vec!('1', '2', '4', '8', 'w', 'r', 'd', 'u', 'x');
	let mut legality = true;
	for character in rw_str.chars() {
		if !(legal.contains(&character)) {
			legality = false;
		}

	}
	if !legality {
		println!("Invalid format speicfied: Junk characters");		
		return None;
	}

	//must specify the memory op
	let rw_matches : Vec<&str> = rw_str.matches(|x| -> bool { (x=='r') | (x=='w')	} ).collect();
	if rw_matches.len() > 1 {
		println!("Invalid format specified: Multiple instances of 'r' and/or 'w'");
		return None;
	}
	else if rw_matches.len() == 0 {
		println!("Invalid format specified: No specification of 'r' or 'w'");
		return None;
	}
	else {
		rw_fmt.rw = rw_matches[0].to_string();
	}

	//format is hex by default. But they can override
	let base_matches: Vec<&str> = rw_str.matches(|x| -> bool { (x=='x') | (x=='u') | (x=='d')	} ).collect();
	if base_matches.len() > 1 {
		println!("Invalid format specified: Multiple instances of 'd', 'u', and/or 'x'");
		return None;	
	}
	else if base_matches.len() == 1 {
		rw_fmt.format = base_matches.iter().map(|x| {
				if (*x == "x") {"d"}
				else if (*x == "u") {"u"}
				else {"d"}
			}).collect::<Vec<&str>>()[0].to_string();
	}

	//size is nume bytes of usize by default but they can overwride.
	let size_matches : Vec<&str> = rw_str.matches(|x| -> bool { (x=='1') | (x=='2') | (x=='4') | (x=='8')} ).collect();
	if rw_matches.len() > 1 {
		println!("Invalid format specified: Multiple instances of size specifiers");
		return None;
	}
	else if size_matches.len() == 1 {
		rw_fmt.n_bytes = size_matches[0].parse::<u8>().unwrap();
	}
	
	Some(rw_fmt)
}


