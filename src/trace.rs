use serde_derive::{Serialize,Deserialize};
use serde_json::to_writer;
use std::collections::HashMap;

use nix::sys::{ptrace,wait};

use std::fs::File;
use std::io::{BufReader,BufRead,Write,Read};

use crate::debugger::Debugger;
use crate::misc::{regs_to_dict,dict_to_regs};
use crate::breakpoint::breakpoint;

#[derive(Serialize,Deserialize)]
pub struct Trace<'a> {
	//registers values when snapshot command was issued
	#[serde(borrow)]
	regs: HashMap<&'a str, u64>,
	//stack and heap trace
	stack: Vec<u64>,
	heap: Vec<u64>,

	
	//reg values after each syscall. Dunno how to structure to include scratch buffers
	//syscalls: Vec<HashMap<&str, u64>>,
	
	//scratch buffers - vector to match syscalls
	//not all syscalls take a pointer to some buffer ( so option)
	//but if they do, there can also be several buffers! (not necessarily just one)
	//so the option contains a vector of vectors.

	//sys_bufs: Vec<Option<Vec<Vec<u64>>>>

	//cur_index: usize,
}

//need scratch buffers

impl<'a> Trace<'a> {

	pub fn New() -> Self {
		Trace {
			regs: HashMap::new(),
			stack: Vec::new(),
			heap: Vec::new(),
		}
	}

	pub fn stack_append(&mut self, val: u64) {
		self.stack.push(val);
	}

	pub fn get_stack(&self) ->  &Vec<u64> {
		&self.stack
	}

	pub fn get_heap(&self) -> &Vec<u64> {
		&self.heap
	}


	pub fn heap_append(&mut self, val: u64) {
		self.heap.push(val);
	}

	pub fn set_trace_regs(&mut self, regs: HashMap<&'a str,u64>) {
		self.regs = regs;
	}

	pub fn get_trace_regs(&self) -> &HashMap<&str, u64> {
		&self.regs
	}

	
	pub fn trace_init(dbg: &mut Debugger) {
		//create the actual trace
		let mut trace_var = Trace::New();
		let regs = match ptrace::getregs(dbg.m_pid) {
			Ok(val) => val,
			Err(err_num) => {
				println!("Failed to retrieve registers with ptrace.\n Error code was {}", err_num);
				return;
			}
		};
		trace_var.set_trace_regs(regs_to_dict(regs));

		let addr_maps = get_heap_and_stack(dbg);
		for map in addr_maps {
			let (addr_start, addr_end) = match map  {
				addr_mapping::Stack(start,end) => (start,end),
				addr_mapping::Heap(start,end) => (start,end),
			};
			
			//we will be reading a word at a time
			let range = (addr_end-addr_start)/8;
			for i in (0..range) {
				let mem_val = dbg.read_mem(addr_start + i*8).unwrap();
				
				match map {
					addr_mapping::Stack(_,_) => {
						trace_var.stack_append(mem_val);
					},
					addr_mapping::Heap(_,_) => {
						trace_var.heap_append(mem_val);
					},
				};
			}		
		}
				
		let mut actual_trace_file = File::create("trace").unwrap();
		let json = serde_json::to_writer(&actual_trace_file, &trace_var).unwrap();
		
		dbg.trace_file = trace_var;
		dbg.trace_enabled = true;
	}


	pub fn restore(file: &mut File, dbg: &Debugger) {
		//might need tow rap the fie in a BufReader
		let mut temp_str = String::new();
		file.read_to_string(&mut temp_str).unwrap();
		let trace : Trace = serde_json::from_str(&temp_str).unwrap();

		let addr_maps = get_heap_and_stack(dbg);
		
		let mut stack_start =0;
		let mut heap_start =0;

		for map in addr_maps {
			
			match map { 
				addr_mapping::Stack(start,end) => {
					stack_start = start;
				},
			//bug here. Can ahve several heap mappings which can rewrite this several times
			//as a whole, trace doesnt take into account several heap sections - it naively assumes theres just one
			//in the trace struct, the heap vec can take in the values for all heaps
			//but in restoring it it doesnt handle it properly.

				addr_mapping::Heap(start,end) => {
					heap_start = start;
				},
			};
		}
		if stack_start != 0 {
			for (idx, stack_val) in trace.get_stack().iter().enumerate() {
				dbg.write_mem(stack_start, *stack_val);
			}
		}
		else {
			println!("Unable to restore stack!");
		}

		if heap_start != 0 {
			for (idx, heap_val) in trace.get_heap().iter().enumerate() {
				dbg.write_mem(heap_start, *heap_val);
			}
		}
		else {
			println!("Unable to restore heap!");
		}

		//INCREDIBLY HARDCODED, JUST TESTING FOR SOURCE OF BUG
		let mut bp = breakpoint::New(dbg.m_pid, 0x55555555514d);
		bp.enable().unwrap();
		ptrace::cont(dbg.m_pid, None);
		wait::waitpid(dbg.m_pid, None);
		bp.disable().unwrap();

		ptrace::setregs(dbg.m_pid, dict_to_regs(trace.get_trace_regs()));
	
		//pray that it works
	}
}

#[derive(PartialEq)]
pub enum addr_mapping {
	Stack(usize,usize),
	Heap(usize,usize),
}


pub fn get_heap_and_stack(dbg: &Debugger) -> Vec<addr_mapping> {
	let mut addr_maps : Vec<addr_mapping> = Vec::new();

	//accessing the maps pseudofile for the debugee process
	let file_path = String::from("/proc/") + &dbg.m_pid.to_string() + &String::from("/maps");
	let file = File::open(&file_path).unwrap();
	let buf_r = BufReader::new(file);
	
	for line_res in buf_r.lines() {
		//want to catch the 'stack' and 'heap' mappings
		let line = line_res.unwrap();
		if line.contains("stack") || line.contains("heap") {	
			//proc_maps entry starts with smn like: 23120000-24220000 [junk]
			//which we re grabbing
			let mem_range : Vec<&str>  = line.split(" ").collect();
			let mem_range = mem_range[0];
			
			//then turning into actual numbers
			//index 0 is start address, index 1 is end address
			let mem_vals : Vec<&str> = mem_range.split("-").collect();
			let mem_vals : Vec<usize> = mem_vals.iter().map(|x| {usize::from_str_radix(x,16).unwrap()}).collect();
			let addr_start = mem_vals[0];
			let addr_end = mem_vals[1];
			
			if line.contains("stack") {
				addr_maps.push(addr_mapping::Stack(addr_start,addr_end));
			}
			else {
				addr_maps.push(addr_mapping::Heap(addr_start,addr_end));
			}
		}
	}
	addr_maps
}
