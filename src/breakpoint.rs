use nix::errno::Errno;
use nix::unistd::Pid;
use nix::sys::ptrace;
use core::ffi::c_void;

#[derive(Clone)]
#[derive(Copy)]
#[derive(Debug)]
pub struct breakpoint {
	saved_data: u8,
	pub addr: usize,
	pub enabled: bool,
	pub pid: Pid,
}

impl breakpoint {
	pub fn New(proc: Pid, target_addr: usize) -> breakpoint {
		breakpoint {
			saved_data: 0,
			addr: target_addr,
			enabled: false,
			pid: proc,
		}
	}

	pub fn enable(&mut self) -> Result<(), Errno> {
		let mut data = match ptrace::read(self.pid, self.addr as *mut c_void) {
			Ok(mem_val) => mem_val as u64,
			Err(err_num) => return Err(err_num),
		};
	
		self.saved_data = (data & 0xff) as u8;
		//oxcc is byte for breakpoints
		data = (data & !0xff) | 0xcc;
		
		unsafe { ptrace::write(self.pid, self.addr as *mut c_void, data as *mut c_void); }
		
		self.enabled = true;
		Ok(())
	}

	pub fn disable(&mut self) -> Result<(),Errno> {
		let mut data = match ptrace::read(self.pid, self.addr as *mut c_void) {
			Ok(mem_val) => mem_val as u64,
			Err(err_num) => return Err(err_num),
		};

		data = (data & !0xff) | (self.saved_data as u64);
		
		unsafe { ptrace::write(self.pid, self.addr as *mut c_void, data as *mut c_void); }
		
		self.enabled = false;
		Ok(())
	}
}

//currently unimplemented
//How we store breakpoints. Was intiially a hashmap of addresses and breakpoints
//But that made it inconvenient to reference breakpoints (have to type whole address to enable/disable/delete/do anything)
//so, vector of breakpoints instead (can access via index
//The second vector (addr_list) is to preent duplicate breakpoints (original reason for choosing hashmap)
pub struct bp_storage {
	pub bp_list: Vec<Option<breakpoint>>,
	pub addr_list: Vec<usize>,
}

impl bp_storage {
	pub fn New() -> bp_storage {
		bp_storage {
			bp_list: Vec::new(),
			addr_list: Vec::new(),
		}
	}


	pub fn insert(&mut self, bp: breakpoint) -> Result<(),()> {
		//breakpoint already set at that address
		if self.addr_list.contains(&bp.addr) {
			return Err(());
		}
		self.bp_list.push(Some(bp));
		self.addr_list.push(bp.addr);
		Ok(())
	}


	pub fn delete(&mut self, idx: usize) -> Result<(),()> {
		if idx > (self.bp_list.len()-1) {
			return Err(());
		}
		match self.bp_list[idx] {
			None => {
				return Err(());
			},
			Some(mut bp) => {
				bp.disable();
				self.bp_list[idx] = None;	
				self.addr_list[idx] = 0;
			},
		}
		Ok(())
	}

	pub fn enable(&mut self, idx: usize) -> Result<(),()> {
		if idx > (self.bp_list.len()-1) {
			return Err(());
		}
		match self.bp_list[idx] {
			None => {
				return Err(());
			},
			Some(mut bp) => {
				bp.enable();
				self.bp_list[idx] = Some(bp);
			},
		}
		Ok(())
	}

	pub fn disable(&mut self, idx: usize) -> Result<(),()> {
		if idx > (self.bp_list.len()-1) {
			return Err(());
		}
		match self.bp_list[idx] {
			None => {
				return Err(());
			},
			Some(mut bp) => {
				bp.disable();
				self.bp_list[idx] = Some(bp);
			},
		}
		Ok(())
	}

	pub fn contains(&self, idx: usize) -> bool {
		if self.addr_list.contains(&idx) {
			return true;
		}
		false
	}
}
