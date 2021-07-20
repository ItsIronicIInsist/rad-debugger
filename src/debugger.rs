use nix::unistd::Pid;
use nix::sys::{wait, ptrace};
use rustyline::Editor;

use std::collections::HashMap;

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

	pub fn run(&mut self) {
		//let waitPidFlag = 1
		//not sure if this returns only when child has exited, or any state transition
		//want to wait for it to setup. When its finished it'll send a SIGTRAP thingy
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
				let mut addr = 0; 
				if args[1].starts_with("0x") {
					match (<usize>::from_str_radix(args[1].strip_prefix("0x").unwrap(), 16)) {
						Ok(num) => {
							addr = num;
						},
						Err(_) => {
							println!("Invalid breakpoint address!");
							return;
						},
					};
				}
				else {
					match args[1].parse::<usize>() {
						Ok(num) => {
							addr = num;
						},
						Err(_) => {
							println!("Invalid breakpoint address!");
							return;
						},
					};
				}
				self.create_breakpoint(addr);
			},
			_ => {println!("Invalid command");},
		};

	}

	fn create_breakpoint(&mut self, address: usize) {
		let mut bp = breakpoint::New(self.m_pid, address);
		bp.enable();
		self.bp_table.entry(address).or_insert(bp);
	}	
}

