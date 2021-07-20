use std::env;
use std::ffi::{CString};
use nix::unistd::{fork,ForkResult,execv};
use nix::sys::ptrace;

use linux_personality::personality;

mod debugger;
mod breakpoint;

use debugger::*;

fn main() {
	let args: Vec<String> = env::args().collect();
	
	if args.len() != 2 {
		panic!("Correct argument usage: debugger <debugee>");
	}

	let prog_name: String = args[1].clone();
	match unsafe{fork()} {
		//we da child
		Ok(ForkResult::Child) => {
			//might need to catch the result and check for errors?
			//Child must call this to make it 'traceable' by the parent
			ptrace::traceme();	
			//runs debugee.  spawn() would fork. 
			let prog = CString::new(prog_name).unwrap();
			personality(linux_personality::ADDR_NO_RANDOMIZE).unwrap();
			execv(&prog, &[prog.clone()]);

		},

		//we da parent
		//child is type Pid
		Ok(ForkResult::Parent {child}) => {
			let mut dbg = Debugger::New(child);
			dbg.run();
		},

		//we da error
		//bruh
		Err(err) => {
			panic!("Failed to fork. Err is {}", err);
		},
	};
}
