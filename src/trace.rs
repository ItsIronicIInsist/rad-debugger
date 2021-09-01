use serde_derive::{Serialize,Deserialize};
use std::collections::HashMap;


#[derive(Serialize,Deserialize)]
pub struct trace<'a> {
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

impl<'a> trace<'a> {

	pub fn New() -> Self {
		trace {
			regs: HashMap::new(),
			stack: Vec::new(),
			heap: Vec::new(),
		}
	}

	pub fn stack_append(&mut self, val: u64) {
		self.stack.push(val);
	}


	pub fn heap_append(&mut self, val: u64) {
		self.heap.push(val);
	}

	pub fn set_trace_regs(&mut self, regs: HashMap<&'a str,u64>) {
		self.regs = regs;
	}

	//dont expose the syscalls or sys_bufs variables (+cur_index)
	/*pub fn restore_values(pid: Pid) {
	
	}*/


}
