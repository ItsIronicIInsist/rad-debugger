use nix::unistd::Pid;
use nix::errno::Errno;
use nix::sys::{wait, ptrace};
use rustyline::{Editor,Helper};

use std::collections::HashMap;
use std::fs::{File};
use std::io::{BufReader,BufRead,Write};

use core::ffi::c_void;
use core::ops::Range;

use crate::breakpoint::{breakpoint, bp_storage};
use crate::misc::*;
use crate::format::*;
use crate::dwarf_functionality::{get_func_from_pc, line_stuff};
use crate::trace::Trace;

use gimli::read::Dwarf;

use serde_json::{to_string};

pub enum dbg_cmd {
	Exit,
	Restart,
	Continue,
}

pub struct Debugger<'a> {
	pub m_pid: Pid, //pid of child
	bp_table: bp_storage,
	pub trace_file: Trace<'a>,
	pub trace_enabled: bool,
}


impl Debugger<'_> {
	pub fn New(child: Pid) -> Debugger<'static> {
		Debugger {
			m_pid: child,
			bp_table: bp_storage::New(),
			trace_file: Trace::New(),
			trace_enabled: false,
		}
	}

	
	//more for internal use rather than ofr direct handling of user commands
	fn get_reg(pid: Pid, reg: &str) -> Result<u64, Errno> {
		let regs =  match ptrace::getregs(pid) {
			Ok(regs_val) => regs_val,
			Err(err_num) => return Err(err_num),
		};
		let regs : HashMap<&str, u64> = regs_to_dict(regs);
		Ok(regs[reg])
	}

	fn set_reg(pid: Pid, reg: &str, val: u64) -> Result<(), Errno> {
		let regs =  match ptrace::getregs(pid) {
			Ok(regs_val) => regs_val,
			Err(err_num) => return Err(err_num),
		};
		let mut regs : HashMap<&str, u64> = regs_to_dict(regs);
		regs.insert(reg , val);
		let regs = dict_to_regs(&regs); 
		
		match ptrace::setregs(pid, regs) {
			Ok(_) => Ok(()),
			Err(err_num) => Err(err_num),
		}
	}

	pub fn read_mem(&self, target_addr: usize) -> Result<u64, Errno> {
		match ptrace::read(self.m_pid, target_addr as *mut c_void) {
			Ok(mem_val) => Ok(mem_val as u64),
			Err(err_num) => Err(err_num),
		}
	}

	pub fn write_mem(&self, target_addr: usize, data: u64) {
		unsafe { ptrace::write(self.m_pid, target_addr as *mut c_void, data as *mut c_void); }
	}

	
	//the bool returned indicates if debugger is to restart
	//the restart and exit commands are handled through this return statement
	pub fn run<T: Helper>(&mut self, inputHandler: &mut Editor::<T>, dwarf_info: &Dwarf<gimli::EndianSlice<gimli::RunTimeEndian>>) -> bool {
		//wait for child to startup. It sends signa when its finished setting up
		wait::waitpid(self.m_pid, None);
		//setting up rustyline 
	
		loop {
			let inputLine = inputHandler.readline("dbg> ");
			let inputLine = match inputLine {
				Ok(line) => {line},
				Err(_) =>  {panic!("Error reading input");},
			};
			//might seem counterintuitve to add command to history before handling it
			//but if we dont then in the match statement for exit and restart we'd have to add the exit/restart to the history inside the arm
			//we record all commands to history, not just valid ones (so that small typos can be recorded and fixed)
			//so its fine. We do need to clone though because it moves the value into the editor
			inputHandler.add_history_entry(inputLine.clone());
			match self.handle_command(&inputLine, dwarf_info) {
				dbg_cmd::Continue => {},
				dbg_cmd::Exit => {return false;},
				dbg_cmd::Restart => {return true;},
			};
		}
	}

	fn handle_command(&mut self, command: &str, dwarf_info: &Dwarf<gimli::EndianSlice<gimli::RunTimeEndian>>) -> dbg_cmd {
		let tmp : Vec<&str> = command.split(' ').collect();
		let mut args : Vec<&str> = vec!();
		for arg in tmp {
		//empty arg
			if arg == "" {
				continue;
			}
			args.push(arg.trim());	
		}
		if args.len() == 0 {
			return dbg_cmd::Continue;
		}
		let command = args[0];


		let mut dbg_result = dbg_cmd::Continue;
		//all commands currently supported
		match command {
			//sent continue signal and wait for next pid
			"continue" | "cont" | "c" => {
				self.continue_exec();
			},
			"break" | "breakpoint" | "b" => {
				self.handle_breakpoints(args);
			},
			"registers" | "regs" | "r" => {
				self.handle_regs(args);
			},
			"memory" | "mem" | "m" => {
				self.handle_mem(args);
			},
			"s" | "si" => {
				ptrace::step(self.m_pid, None);
			},
			"exit" => {
				dbg_result = dbg_cmd::Exit;
			},
			"restart" => {
				dbg_result = dbg_cmd::Restart;
			},
			"dwarf" => {
				match get_func_from_pc(dwarf_info, 0x113e) {
					Some(die) => {println!("{:?}",die);},
					None => {},
				}
				line_stuff(dwarf_info);
			},
			"snapshot"=> {
				Trace::trace_init(self);	
			},
			"restore" => {
				self.restore_trace_entrance(args);
			},
			_ => {println!("Invalid command");},
		};
		dbg_result
	}

	fn restore_trace_entrance(&self, args: Vec<&str>) {
		if args.len() < 2 {
			println!("File must be specified");
		}

		let mut file = match File::open(args[1]) {
			Ok(result) => result,
			Err(err)=> {
				println!("error in opening file. Error was {:?}", err.kind()); 
				return;
			},
		};

		Trace::restore(&mut file, self);
	}


	fn continue_exec(&mut self) {
		//get value of pc
		let addr =  (Debugger::get_reg(self.m_pid, "rip").unwrap() - 1) as usize;
		
		//check if there is a breakpoint at that addr
		if self.bp_table.addr_list.contains(&addr) {
			//cant search for it smartly so just check every breakpoint (sadly)
			for bp_opt in &mut self.bp_table.bp_list {
				let mut bp;
				match bp_opt {
					None => {
						continue;
					},
					Some(bp_tmp) => {
						bp=bp_tmp;
					},
				}
				//found the brekapoint at the address but its already disable. All good
				if bp.addr == addr && bp.enabled == false {
					break;
				}
				//found breakpooint and its enabed, so need to step around it
				else if bp.addr == addr && bp.enabled == true {
					bp.disable();
					Debugger::set_reg(self.m_pid, "rip", addr as u64);
					ptrace::step(self.m_pid, None);
					wait::waitpid(self.m_pid, None);
					bp.enable();
					break;
				}
			}
		}
		else {
			Debugger::set_reg(self.m_pid, "rip", addr as u64);
		}
		ptrace::cont(self.m_pid, None);
		wait::waitpid(self.m_pid, None);

		println!("{:?}", ptrace::getsiginfo(self.m_pid).unwrap());
	}

	fn handle_breakpoints(&mut self, args: Vec<&str>) {
	//b list | l
	//b disable | d <idx>
	//b enabled | e <idx>
	//b delete | de <idx>
	//b <idx>
		if args.len() < 2 {
			println!("Breakpoint command needs second argument");
			return;
		}

		let mut idx = 0;
		//the commands which need an index specified
		//need to check its a valid index (or theres on at all)
		match args[1] {
			"disable" | "d" | "enable" | "e" | "delete" | "de" => {
				if args.len() < 3 {
					println!("Need address to be specified");
				}
				match str_to_int(args[2]) {
					Some(num) => {
						idx = num;
					},
					None => {
						println!("Invalid index specified");
						return;
					},
				};
			},
			_ => {},
		};
		
		match args[1] {
			"list" | "l" => {
				self.list_breakpoints();
			},
			"disable" | "d" => {
				if args.len() < 3 {
					println!("Need index specified to disable breakpoint");
				}
				match self.bp_table.disable(idx) {
					Ok(_) => {},
					Err(_) => {
						println!("Error disabling breakpoint. Invalid index.");
					},
				};
			},
			"enable" | "e" => {
				if args.len() < 3 {
					println!("Need index specified to enable breakpoint");
				}
				match self.bp_table.enable(idx) {
					Ok(_) => {},
					Err(_) => {
						println!("Error enabling breakpoint. Invalid index.");
					},
				};
			},
			"delete" | "de" => {
				if args.len() < 3 {
					println!("Need index specified to delete breakpoint");
				}
				match self.bp_table.delete(idx) {
					Ok(_) => {},
					Err(_) => {
						println!("Error deleting breakpoint. Invalid index.");
					},
				};
			},
			//Default behaviour is to create a breakpoint (if no other command is given)
			_  => {
				self.create_breakpoint(args);
			}
		}
	}


	fn create_breakpoint(&mut self, args: Vec<&str>) {
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
		match bp.enable() {
			Ok(_) => {},
			Err(err_num) => {
				println!("Failed to enable breakpoint.\n Ptrace read request failed with err {}", err_num);
				return;
			},
		}
		match self.bp_table.insert(bp) {
			Ok(_) => {},
			Err(_) => {
				println!("Error creating breakpoint. There is already a breakpoint at that address");
			},
		};
	}

	//list of breakpoints (their addresses and if they're enbaled
	fn list_breakpoints(&self) {
		println!("<idx>: <set?>: <addr>");
		for (idx, bp_maybe) in self.bp_table.bp_list.iter().enumerate() {
			match bp_maybe {
				Some(bp) => {
					println!("<{}>: <{}>: <{:#x}>", idx, bp.enabled, bp.addr); 
				},
				None => {},
			};
		}
	}

	fn handle_regs(&self, args: Vec<&str>) {
		let regs = match  ptrace::getregs(self.m_pid) {
			Ok(regs_val) => regs_val,
			Err(err_num) => {
				println!("Failed to retrieve registers with ptrace.\n Error code was {}", err_num);
				return;
			},
		};
		let regs : HashMap<&str, u64> = regs_to_dict(regs);
	
		//just dumping register
		if args.len() < 2 {
			dump_regs(regs);
			return;
		}
		
		//function for formatting rw
		//the rw_format struct explains it better (in format.rs)
		let mut fmt;
		match format_rw(args[1]) {
			Some(f) => {
				fmt = f;
			},
			None => {
				return
			},
		}

		//reading specific registers (not dumping all)
		//args can be any number larger than two (we list all args specified in command
		if fmt.rw == "r" && args.len() > 2 {

			//looping through each reg spicied
			for target_reg in args[2..].into_iter(){
				let mut reg_val = 0; 
				//get the value of register specified
				match regs.get(target_reg) {
					Some(num) => {
						reg_val = *num;
					},
					None => {
						//if  thee is an invalid or mispelt register, just continue
						continue;
					}
				}

				//need to fix casting issue. Maybe just cast as i64 and do format string stuff to truncate?
				reg_val = fmt.trim_val(reg_val);
				match (fmt.format).as_str() {
					"d" => {println!("{}: {}", target_reg, reg_val);},//cast!(reg_val, fmt.n_bytes));},
					"u" => {println!("{}: {}", target_reg, reg_val);}, //cast!(reg_val,-fmt.n_bytes ));},
					// "x" is redundant, just explicitly showing hex is default case
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
			let mut orig_reg_val = 0; 
			//get the value of register specified
			match regs.get(args[2]) {
				Some(num) => {
					orig_reg_val = *num;
				},
				None => {
					println!("Invalid register name");
					return;
				}
			}

			//takes user num. Trims it so its only n-bytes wide
			//If we are writing one byte, want to preserve the other 7 bytes of the reg
			
			//te right side is complicated because of edge cases
			//but it takes the original register value and &'s it with a bitmask
			//the bitmask starts of as u64::MAX (so no bits change)
			//but depending on how many bytes are written. If 1 byte are written, it cleares the bottom byte off the mask
			//the power seciton is used instead of bitshifting the u64::MAX because u64::MAX << 64 is 'undefined' 
			//and that scenario occurs when the user specifies 8 bytes (e.g no bitmask)
			println!("{:?}", fmt);
			let modified_val = fmt.trim_val(user_num) | (orig_reg_val & (u64::MAX -  ( (2u128.pow((fmt.n_bytes*8) as u32) -1) as u64) ) );
			match Debugger::set_reg(self.m_pid, args[2], modified_val) {
				Ok(_) => {},
				Err(err_num) => {
					println!("Failed to write to register.\n Error in ptrace request. Error code was {}", err_num);
				}
			}
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
					println!("No address specified");
					return;
				},
			};

			let mut mem_val = match self.read_mem(addr) {
				Ok(mem) => mem,
				Err(err_num) => {
					println!("Failed to read memory value from address.\n Error in ptrace request. Error code was {}", err_num);
					return;
				},
			};
			mem_val = fmt.trim_val(mem_val);
			match (fmt.format).as_str() {
				"d" => {println!("{:#x}: {}", addr, mem_val);},//cast!(reg_val, fmt.n_bytes));},
				"u" => {println!("{:#x}: {}", addr, mem_val);}, //cast!(reg_val,-fmt.n_bytes ));},
				//or "x" is redundant, just explicitly showing hex is default case
				_ | "x" => {println!("{:#x}: {:#x}", addr, mem_val);},//cast!(reg_val, fmt.n_bytes);},
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
					println!("No address specified");
					return;
				}
			};

			let mut user_num = 0;
			match str_to_int(args[3]) { 
				Some(num) => {
					user_num = num as u64;
				} 
				None => {
					println!("Invalid value to set memory to!!");
					return;
				}
			};

			let orig_mem_val = match self.read_mem(addr) {
				Ok(mem) => mem,
				Err(err_num) => {
					println!("Failed to read memory value from address, as to prepare for write.\n Error in ptrace request. Error code was {}", err_num);
					return;
				},
			};

			//takes user num. Trims it so its only n-bytes wide
			//If we are writing one byte, want to preserve the other 7 bytes of the reg
			//WHich is second part. the & and bitshift part
			//creates a mask that it 0x1111111100000000 (n-bytes of 0's, the rest are 1's(
			//which empties out the bottom n-bytes opf the reg
			let modified_val = fmt.trim_val(user_num) | (orig_mem_val & (u64::MAX -  ( (2u128.pow((fmt.n_bytes*8) as u32) -1) as u64) ) );

			self.write_mem(addr, modified_val);
		}
	}
}

