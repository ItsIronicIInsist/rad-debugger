use libc::{user_regs_struct, c_ulonglong};
use std::collections::HashMap;


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



pub fn dict_to_regs(dict: &HashMap<&str, u64>) -> user_regs_struct {
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

pub fn dump_regs(dict: HashMap<&str, u64>) {
	println!("rax: {:#x}", *dict.get("rax").unwrap());
	println!("rbx: {:#x}", *dict.get("rbx").unwrap());
	println!("rcx: {:#x}", *dict.get("rcx").unwrap());
	println!("rdx: {:#x}", *dict.get("rdx").unwrap());
	println!("rdi: {:#x}", *dict.get("rdi").unwrap());
	println!("rsi: {:#x}", *dict.get("rsi").unwrap());
	println!("rbp: {:#x}", *dict.get("rbp").unwrap());
	println!("rsp: {:#x}", *dict.get("rsp").unwrap());
	println!("rip: {:#x}", *dict.get("rip").unwrap());
	println!("r8: {:#x}", *dict.get("r8").unwrap());
	println!("r9: {:#x}", *dict.get("r9").unwrap());
	println!("r10: {:#x}", *dict.get("r10").unwrap());
	println!("r11: {:#x}", *dict.get("r11").unwrap());
	println!("r12: {:#x}", *dict.get("r12").unwrap());
	println!("r13: {:#x}", *dict.get("r13").unwrap());
	println!("r14: {:#x}", *dict.get("r14").unwrap());
	println!("r15: {:#x}", *dict.get("r15").unwrap());
	println!("eflags: {:#x}", *dict.get("eflags").unwrap());
	println!("cs: {:#x}", *dict.get("cs").unwrap());
	println!("ds: {:#x}", *dict.get("ds").unwrap());
	println!("es: {:#x}", *dict.get("es").unwrap());
	println!("fs: {:#x}", *dict.get("fs").unwrap());
	println!("gs: {:#x}", *dict.get("gs").unwrap());
	println!("ss: {:#x}", *dict.get("ss").unwrap());
	println!("gs_base: {:#x}", *dict.get("gs_base").unwrap());
	println!("fs_base: {:#x}", *dict.get("fs_base").unwrap());
	println!("orig_rax: {:#x}", *dict.get("orig_rax").unwrap());
}
