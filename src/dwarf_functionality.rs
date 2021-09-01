use std::ops::Range;

use gimli::read::Dwarf;
use gimli::{DwAt,EndianSlice,RunTimeEndian,AttributeValue,DebuggingInformationEntry};

//Entire dwarf object
pub fn get_func_from_pc<'a, R: gimli::Reader>(dwarf_info: &'a Dwarf<R>,pc:u64) -> Option<gimli::EntriesCursor<R>> {
//pub fn get_func_from_pc<'a>(dwarf_info: &'a Dwarf<EndianSlice<RunTimeEndian>>,pc:u64) -> Option<&'a DebuggingInformationEntry<'a, 'a, EndianSlice<'a, RunTimeEndian>>> {
	//tterable of each compilation unit
	let mut compilation_units = dwarf_info.units();

	while let Ok(compilation_unit_opt) = compilation_units.next() {
		let compilation_unit = match compilation_unit_opt {
			Some(unit) => {
				unit
			},
			//All units have been run through.
			None => {
				break;
			},
		};
		let abbreviations = dwarf_info.abbreviations(&compilation_unit).unwrap();
		//for the compilation unit, get all the DIE's
		let mut DIE_tree = compilation_unit.entries(&abbreviations);

		
		//loop through them
		//Returns None when n more entries
		while DIE_tree.next_entry().unwrap() == Some(()) {
			let cur_DIE = match DIE_tree.current() {
				Some(DIE_entry) => {
					DIE_entry
				},
				//empty DIE entry
				None => {
					continue;
				},
			};
			
			//i should also covere inlined stuff 
			if cur_DIE.tag() != gimli::DW_TAG_subprogram {
				continue;
			}
			let func_range = match get_pc_range(cur_DIE) {
				Some(range) => {range},
				None => {continue;},
			};

			if func_range.contains(&pc) {
				//get name attribute
				return None;
			//	return Some(DIE_tree);
				//return Some(cur_DIE);
			}
		}
	}
	return None;
}


fn get_pc_range<R: gimli::Reader>(func: &DebuggingInformationEntry<R>) -> Option<Range<u64>> {
	//DwAt(0x11) = DW_AT_low_pc
	let low_pc = match func.attr(DwAt(0x11)).unwrap() {
		Some(attr) => {
			let var = match attr.value() {
				AttributeValue::Addr(num) => {num},
				_ => {return None;},
			};
			var
		},
		None => {
			return None;
		},
	};
	
	//DwAt(0x12) = DW_AT_high_pc
	let high_pc = match func.attr(DwAt(0x12)).unwrap() {
		Some(attr) => {
			match attr.udata_value() {
				Some(attr_u64) => {
					attr_u64
				},
				None => {
					return None;
				},
			}
		},
		None => {
			return None;
		},
	};
	Some(Range { start: low_pc, end: low_pc+high_pc})
}




pub fn line_stuff<R: gimli::Reader<Offset=usize>>(dwarf_info: &Dwarf<R>) {
	let incomplete_prog = dwarf_info.debug_line.program(gimli::DebugLineOffset(0 as usize),8,None,None).unwrap();
	let (complete_prog, seqs) = incomplete_prog.sequences().unwrap();
	for sequence in seqs {
		let mut linerows = complete_prog.resume_from(&sequence);
		while let Ok(linerow_opt) = linerows.next_row() {
			let (prog_header, linerow) = match linerow_opt {
				Some((header_tmp, row_tmp)) => {(header_tmp,row_tmp)},
				None =>{break;},
			};
			println!("address is {:#x}", linerow.address());
			let file = match linerow.file(prog_header) {
				Some(file_entry) => {file_entry},
				None => {continue;},
			};
			let a = match file.path_name() {
				gimli::AttributeValue::DebugLineStrRef(offset) => {offset},
				_ => {continue;},
			};

			println!("Owner file is {:?}",dwarf_info.line_string(a).unwrap().to_string().unwrap() );

		}
	}
	
}
