use nix::unistd::Pid;
use nix::sys::{wait, ptrace};
use rustyline::Editor;

use libc::{user_regs_struct, c_ulonglong};

use std::collections::HashMap;
use std::mem::size_of;

use core::ffi::c_void;

use crate::breakpoint::breakpoint;


pub struct Debugger {
	pub m_pid: Pid, 
	bp_table: HashMap<usize, breakpoint>,
}


impl Debugger {
	pub fn New(child: Pid) -> Debugger {
		Debugger {
			m_pid: child,
			bp_table: HashMap::new(),
		}
	}
	
	//more for internal use rather than ofr direct handling of user commands
	fn get_reg(&self, reg: &str) -> u64 {
		let regs =  ptrace::getregs(self.m_pid).unwrap();
		let regs : HashMap<&str, u64> = regs_to_dict(regs);
		return regs[reg];
	}

	fn set_reg(&self, reg: &str, val: u64) {
		let regs =  ptrace::getregs(self.m_pid).unwrap();
		let mut regs : HashMap<&str, u64> = regs_to_dict(regs);
		regs.insert(reg , val);
		let regs = dict_to_regs(regs); 
		ptrace::setregs(self.m_pid, regs).unwrap();
	}

	fn read_mem(&self, target_addr: usize) -> u64 {
		ptrace::read(self.m_pid, target_addr as *mut c_void).unwrap() as u64
	}

	fn write_mem(&self, target_addr: usize, data: u64) {
		unsafe { ptrace::write(self.m_pid, target_addr as *mut c_void, data as *mut c_void); }
	}

	

	pub fn run(&mut self) {
		wait::waitpid(self.m_pid, None);
		let mut inputHandler = Editor::<()>::new();
		loop {
			let inputLine = inputHandler.readline("dbg> ");
			let inputLine = match inputLine {
				Ok(line) => {line},
				Err(_) =>  {panic!("Error reading input");},
			};
			self.handle_command(&inputLine); 
			inputHandler.add_history_entry(inputLine);
		}
	}

	fn handle_command(&mut self, command: &str) {
		let args : Vec<&str> = command.split(' ').collect();
		let command = args[0];

		match command {
			"continue" | "cont" | "c" => {
				ptrace::cont(self.m_pid, None);
				wait::waitpid(self.m_pid, None);
			},
			"break" | "breakpoint" | "b" => {
				self.create_breakpoint(args);
			},
			"registers" | "regs" | "r" => {
				self.handle_regs(args);
			},
			"memory" | "mem" | "m" => {
				self.handle_mem(args);
			},
			_ => {println!("Invalid command");},
		};

	}

	fn create_breakpoint(&mut self, args: Vec<&str>) {
		if args.len() < 2 {
			println!("Breakpoint command needs address");
			return;
		}
		
		let mut addr = 0;
		match str_to_int(args[1]) { 
			Some(num) => {
				addr = num;
			}
			None => {
				println!("Invalid breakpoint address!");
				return;
			}
		};
		
		let mut bp = breakpoint::New(self.m_pid, addr);
		bp.enable();
		self.bp_table.entry(addr).or_insert(bp);
	}


	fn handle_regs(&self, args: Vec<&str>) {
		let regs =  ptrace::getregs(self.m_pid).unwrap();
		let regs : HashMap<&str, u64> = regs_to_dict(regs);
	
		//just dumping register
		if args.len() < 2 {
			for (reg, val) in regs.iter() {
				println!("{}: {:#x}", reg, val);
			}
			return;
		}
		
		//function for formatting rw
		//rw_format struct explains it better
		let mut fmt;
		match format_rw(args[1]) {
			Some(f) => {
				fmt = f;
			},
			None => {
				return
			},
		}


		if fmt.rw == "r" && args.len() > 2 {
			for target_reg in args[2..].into_iter(){
				let mut reg_val = 0; 
				match regs.get(target_reg) {
					Some(num) => {
						reg_val = *num;
					},
					None => {
						println!("Invalid register");
						return;
					}
				}
				reg_val = fmt.trim_val(reg_val);
				match (fmt.format).as_str() {
					"d" => {println!("{}: {}", target_reg, reg_val);},//cast!(reg_val, fmt.n_bytes));},
					"u" => {println!("{}: {}", target_reg, reg_val);}, //cast!(reg_val,-fmt.n_bytes ));},
					//or "x" is redundant, just explicitly showing hex is default case
					_ | "x" => {println!("{}: {:#x}", target_reg, reg_val);},//cast!(reg_val, fmt.n_bytes);},
				};
			}
		}
		
		//writing to a register
		//e.g regs w rax 0x10
		else if fmt.rw== "w"  && args.len() == 4 {
			let mut user_num = 0;
			match str_to_int(args[3]) { 
				Some(num) => {
					user_num = num as u64;
				} 
				None => {
					println!("Invalid value to set register to!");
					return;
				}
			};
			//gets original value of reg
			let orig_reg_val = self.get_reg(args[2]);

			//takes user num. Trims it so its only n-bytes wide
			//If we are writing one byte, want to preserve the other 7 bytes of the reg
			//WHich is second part. the & and bitshift part
			//creates a mask that it 0x1111111100000000 (n-bytes of 0's, the rest are 1's(
			//which empties out the bottom n-bytes opf the reg
			let modified_val = fmt.trim_val(user_num) | (orig_reg_val & (u64::MAX << (fmt.n_bytes*8)));
			self.set_reg(args[2], modified_val);
		}
	}


	fn handle_mem(&self, args: Vec<&str>) {
		if args.len() < 3 {
			println!("Memory command needs to be formatted: mem r/w addr");
			return;
		}

		let mut fmt;
		match format_rw(args[1]) {
			Some(f) => {
				fmt = f;
			},
			None => {
				return
			},
		}

		if fmt.rw == "r" && args.len() == 3 {
			let mut addr = 0;
			match str_to_int(args[2]) {
				Some(num) => {
					addr = num;
				},
				None => {
					return;
				},
			};
			let mut mem_val = self.read_mem(addr);
			mem_val = fmt.trim_val(mem_val);
			match (fmt.format).as_str() {
				"d" => {println!("{}: {}", addr, mem_val);},//cast!(reg_val, fmt.n_bytes));},
				"u" => {println!("{}: {}", addr, mem_val);}, //cast!(reg_val,-fmt.n_bytes ));},
				//or "x" is redundant, just explicitly showing hex is default case
				_ | "x" => {println!("{}: {:#x}", addr, mem_val);},//cast!(reg_val, fmt.n_bytes);},
			};
		}

		//writing to an addr
		//e.g mem w <addr> <val>
		else if fmt.rw == "w" && args.len() == 4 {
			let mut addr = 0;
			match str_to_int(args[2]) { 
				Some(num) => {
					addr = num as usize;
				} 
				None => {
					println!("Invalid value to set register to!");
					return;
				}
			};

			let mut user_num = 0;
			match str_to_int(args[3]) { 
				Some(num) => {
					user_num = num as u64;
				} 
				None => {
					println!("Invalid value to set register to!");
					return;
				}
			};

			let orig_mem_val = self.read_mem(addr);

			//takes user num. Trims it so its only n-bytes wide
			//If we are writing one byte, want to preserve the other 7 bytes of the reg
			//WHich is second part. the & and bitshift part
			//creates a mask that it 0x1111111100000000 (n-bytes of 0's, the rest are 1's(
			//which empties out the bottom n-bytes opf the reg
			let modified_val = fmt.trim_val(user_num) | (orig_mem_val & (u64::MAX << (fmt.n_bytes*8)));

			self.write_mem(addr, modified_val);
		}
	}
}

//supports both hex and decimal string representations
pub fn str_to_int(string: &str) -> Option<usize> {
	if string.starts_with("0x") {
		match (<usize>::from_str_radix(string.strip_prefix("0x").unwrap(), 16)) {
			Ok(num) => {
				Some(num)
			},
			Err(_) => {
				None
			},
		}
	}
	else {
		match string.parse::<usize>() {
			Ok(num) => {
				Some(num)
			},
			Err(_) => {
				None
			},
		}
	}
}


//Use: mem rx2 <addr>. Read, 2 bytes, hex format
//n bytes: 1,2,4,8
//rw: Read or Write. Enum?
//format: x (hex. Default), u (unsigned base-10), d(signed base-10)

//use a HashSet for no duplictes? Still want an easy way to ensure - zero to one formats, one of r/w, zero to one of n-bytes
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
	fn New() -> Self {
		rw_formatting {
			rw: String::new(),
			format: String::new(),
			n_bytes: size_of::<usize>() as u8,
		}
	}

	fn trim_val(&self, val: u64) -> u64 {
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



//horrid, I know. But user_regs_struct implements no form of iterator
//I need to take user input of "register rax", and output the value o rax
//and I dont wwant a 30-long match statement for each register, put in a loop for if several registers need to be printed
//was going to do a macro that returns the fields of a struct as strings, as well as their values (tuple array), but it seems  macros cant do that unless
//you literally give it the struct definition

pub fn regs_to_dict(registers: user_regs_struct) -> HashMap<&'static str, u64> {
	let mut temp : HashMap<&str, u64> = HashMap::new();
	temp.insert("rax", registers.rax as u64);
	temp.insert("rbx", registers.rbx as u64);
	temp.insert("rcx", registers.rcx as u64);
	temp.insert("rdx", registers.rdx as u64);
	temp.insert("rdi", registers.rdi as u64);
	temp.insert("rsi", registers.rsi as u64);
	temp.insert("rbp", registers.rbp as u64);
	temp.insert("rsp", registers.rsp as u64);
	temp.insert("r8", registers.r8 as u64);
	temp.insert("r9", registers.r9 as u64);
	temp.insert("r10", registers.r10 as u64);
	temp.insert("r11", registers.r11 as u64);
	temp.insert("r12", registers.r12 as u64);
	temp.insert("r13", registers.r13 as u64);
	temp.insert("r14", registers.r14 as u64);
	temp.insert("r15", registers.r15 as u64);
	temp.insert("rip", registers.rip as u64);
	temp.insert("eflags", registers.eflags as u64);
	temp.insert("cs", registers.cs as u64);
	temp.insert("ds", registers.ds as u64);
	temp.insert("es", registers.es as u64);
	temp.insert("fs", registers.fs as u64);
	temp.insert("gs", registers.gs as u64);
	temp.insert("gs_base", registers.gs_base as u64);
	temp.insert("fs_base", registers.fs_base as u64);
	temp.insert("ss", registers.ss as u64);
	temp.insert("orig_rax", registers.orig_rax as u64);
	temp
}



pub fn dict_to_regs(dict: HashMap<&str, u64>) -> user_regs_struct {
	user_regs_struct {
		rax: *dict.get("rax").unwrap(),
		rbx: *dict.get("rbx").unwrap(),
		rcx: *dict.get("rcx").unwrap(),
		rdx: *dict.get("rdx").unwrap(),
		rdi: *dict.get("rdi").unwrap(),
		rsi: *dict.get("rsi").unwrap(),
		rbp: *dict.get("rbp").unwrap(),
		rsp: *dict.get("rsp").unwrap(),
		r8: *dict.get("r8").unwrap(),
		r9: *dict.get("r9").unwrap(),
		r10: *dict.get("r10").unwrap(),
		r11: *dict.get("r11").unwrap(),
		r12: *dict.get("r12").unwrap(),
		r13: *dict.get("r13").unwrap(),
		r14: *dict.get("r14").unwrap(),
		r15: *dict.get("r15").unwrap(),
		rip: *dict.get("rip").unwrap(),
		eflags: *dict.get("eflags").unwrap(),
		cs: *dict.get("cs").unwrap(),
		ds: *dict.get("ds").unwrap(),
		es: *dict.get("es").unwrap(),
		fs: *dict.get("fs").unwrap(),
		gs: *dict.get("gs").unwrap(),
		gs_base: *dict.get("gs_base").unwrap(),
		fs_base: *dict.get("fs_base").unwrap(),
		ss: *dict.get("ss").unwrap(),
		orig_rax: *dict.get("orig_rax").unwrap(),
	}
}

