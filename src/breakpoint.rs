use nix::unistd::Pid;
use nix::sys::ptrace;
use core::ffi::c_void;

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

	pub fn enable(&mut self) {
		let mut data = ptrace::read(self.pid, self.addr as *mut c_void).unwrap() as u64;
		
		self.saved_data = (data & 0xff) as u8;
		//oxcc is byte for breakpoints
		data = (data & !0xff) & 0xcc;
		
		unsafe { ptrace::write(self.pid, self.addr as *mut c_void, data as *mut c_void); }
		
		self.enabled = true;
	}

	pub fn disable(&mut self) {
		let mut data = ptrace::read(self.pid, self.addr as *mut c_void).unwrap() as u64;

		data = (data & !0xff) & (self.saved_data as u64);
		
		unsafe { ptrace::write(self.pid, self.addr as *mut c_void, data as *mut c_void); }
		
		self.enabled = false;
	}



}
