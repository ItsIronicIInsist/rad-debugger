use std::env;
use std::ffi::{CString};
use nix::unistd::{fork,ForkResult,execv};
use nix::sys::ptrace;
use nix::errno::Errno;

use linux_personality::personality;

use rustyline::Editor;

mod debugger;
mod breakpoint;
mod misc;
mod format;

use debugger::*;

fn main() {
	let args: Vec<String> = env::args().collect();
	
	if args.len() != 2 {
		panic!("Correct argument usage: debugger <debugee>");
	}

	let prog_name: String = args[1].clone();

	//restart implementation is veyr messy. I did not consider it when implementing how I handled commands
	//both the child and err parts dont ever really return, so that just leaves the parent
	//it returns 0, exiting the loop if user inputs exit command
	//if they enter a restart then it returns 1, but we dont actually need to cover that, since default is to keep restarting
	let mut restart = true;
	//this handles command history. Must be definer outside the loop
	let mut inputHandler = Editor::<()>::new();
	while restart == true {
		match unsafe{fork()} {
			
			Ok(ForkResult::Child) => {
				//might need to catch the result and check for errors?
				//Child must call this to make it 'traceable' by the parent
				ptrace::traceme();	
				let prog = CString::new(prog_name.clone()).unwrap();
				
				//disable aslr
				personality(linux_personality::ADDR_NO_RANDOMIZE).unwrap();
				
				//runs debugee.  spawn() would fork. 
				execv(&prog, &[prog.clone()]);

			},

			//child is type Pid
			Ok(ForkResult::Parent {child}) => {
				let mut dbg = Debugger::New(child);
				if dbg.run(&mut inputHandler) == false  {
					restart = false;
				}
				match ptrace::kill(child) {
					Ok(_) => {},
					Err(err_num) => {
						//ESRCH means process is already dead
						if err_num != Errno::ESRCH {
							println!("Ptrace kill command failed with {}", err_num);
						}
					},
				}
			},

			Err(err) => {
				panic!("Failed to fork. Err is {}", err);
			},
		};
	}
}
