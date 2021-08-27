use std::env;
use std::ffi::{CString};
use std::fs;
use std::borrow::Cow;

use nix::unistd::{fork,ForkResult,execv};
use nix::sys::ptrace;
use nix::errno::Errno;

use linux_personality::personality;

use rustyline::Editor;

use gimli;
use object::{Object, ObjectSection};


mod debugger;
mod breakpoint;
mod misc;
mod format;
mod dwarf_functionality;

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
	
	
	//setting up dwarf debug info - only want to do it once, not every restart
	let bin_data = fs::read(prog_name.clone()).unwrap();
	let obj_file = object::read::File::parse(&*bin_data).unwrap();
	let load_section = |id: gimli::SectionId| -> Result<Cow<[u8]>, gimli::Error> {
		match obj_file.section_by_name(id.name()) {
		//section has been found
			Some(section) => {
				//decompress it
				//can potentially fail to decompress. If so, return empty section
				Ok(section.uncompressed_data().unwrap_or(Cow::Borrowed(&[][..])))
			},
			None => {
				//return empty section
				Ok(Cow::Borrowed(&[][..]))
			},
		}
	};

	let dwarf_cow = gimli::Dwarf::load(&load_section, &load_section).unwrap();


	//this was just grabbed from the docs. ont understand trait objects quite yet, so thisll be a placeholder
	let borrow_section: &dyn for<'a> Fn(
			&'a Cow<[u8]>,
		) -> gimli::EndianSlice<'a, gimli::RunTimeEndian> =
			&|section| gimli::EndianSlice::new(&*section, gimli::RunTimeEndian::Little);
			
	let dwarf = dwarf_cow.borrow(&borrow_section);




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

				


				if dbg.run(&mut inputHandler,&dwarf) == false  {
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
