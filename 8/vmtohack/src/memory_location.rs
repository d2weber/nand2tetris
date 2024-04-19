use crate::asm_generators::*;

pub enum MemoryLocation {
    Constant(usize),
    Local(usize),
    Argument(usize),
    This(usize),
    That(usize),
    Temp(usize),
    Pointer(usize),
    Static(usize),
}

impl MemoryLocation {
    pub fn push(&self, module_id: &str) -> String {
        match self {
            MemoryLocation::Constant(number) => format!("@{number}\nD=A\n{}", push_d()),
            MemoryLocation::Local(offset) => push_from_addr("LCL", *offset),
            MemoryLocation::Argument(offset) => push_from_addr("ARG", *offset),
            MemoryLocation::This(offset) => push_from_addr("THIS", *offset),
            MemoryLocation::That(offset) => push_from_addr("THAT", *offset),
            MemoryLocation::Temp(offset) => push_to(5 + offset),
            MemoryLocation::Pointer(id) => push_to(pointer_name(id)),
            MemoryLocation::Static(id) => push_to(format!("{module_id}.{id}")),
        }
    }

    pub fn pop(&self, module_id: &str) -> String {
        match self {
            MemoryLocation::Constant(_) => panic!("Cannot pop constant"),
            MemoryLocation::Local(offset) => pop_to_addr("LCL", *offset),
            MemoryLocation::Argument(offset) => pop_to_addr("ARG", *offset),
            MemoryLocation::This(offset) => pop_to_addr("THIS", *offset),
            MemoryLocation::That(offset) => pop_to_addr("THAT", *offset),
            MemoryLocation::Temp(offset) => pop_from(5 + offset),
            MemoryLocation::Pointer(id) => pop_from(pointer_name(id)),
            MemoryLocation::Static(id) => pop_from(format!("{module_id}.{id}")),
        }
    }
}

fn pointer_name(pointer_id: &usize) -> &str {
    match pointer_id {
        0 => "THIS",
        1 => "THAT",
        _ => panic!("Invalid pointer id `{pointer_id}`"),
    }
}

impl MemoryLocation {
    pub fn from<'a>(parts: &mut impl Iterator<Item = &'a str>) -> Result<Self, &'static str> {
        let kind = parts.next().ok_or("Missing kind")?;
        let number: usize = parts
            .next()
            .ok_or("Missing number")?
            .parse()
            .map_err(|_| "Could not parse number")?;
        Ok(match kind {
            "constant" => MemoryLocation::Constant(number),
            "local" => MemoryLocation::Local(number),
            "argument" => MemoryLocation::Argument(number),
            "this" => MemoryLocation::This(number),
            "that" => MemoryLocation::That(number),
            "temp" => MemoryLocation::Temp(number),
            "pointer" => MemoryLocation::Pointer(number),
            "static" => MemoryLocation::Static(number),
            _ => return Err("Invalid kind"),
        })
    }
}
