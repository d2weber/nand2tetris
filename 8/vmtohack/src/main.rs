use std::{env, fmt::Display, fs, path::Path, str::FromStr};

fn main() {
    let mut args = env::args();
    if args.len() != 2 {
        panic!("Expect single parameter to `*.vm` file.");
    }
    let filename = args.next_back().unwrap();
    vm_translate(Path::new(&filename));
}

#[cfg(test)]
mod test {
    use std::process::{Command, Stdio};

    use super::*;

    #[test]
    fn simple_add() {
        check_tst("../../7/StackArithmetic/SimpleAdd/SimpleAdd.vm");
    }

    #[test]
    fn stack_test() {
        check_tst("../../7/StackArithmetic/StackTest/StackTest.vm");
    }

    #[test]
    fn basic_test() {
        check_tst("../../7/MemoryAccess/BasicTest/BasicTest.vm");
    }

    #[test]
    fn pointer_test() {
        check_tst("../../7/MemoryAccess/PointerTest/PointerTest.vm");
    }

    #[test]
    fn static_test() {
        check_tst("../../7/MemoryAccess/StaticTest/StaticTest.vm");
    }

    /// Compile provided vm file to asm, and check result with a `*.tst` file
    fn check_tst(vm_file: &str) {
        let cargo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let vm_file = cargo_root.join(vm_file);
        vm_translate(&vm_file);
        assert!(
            Command::new("bash")
                .arg("../../../tools/CPUEmulator.sh")
                .arg(vm_file.with_extension("tst"))
                .current_dir(cargo_root)
                .stdout(Stdio::null())
                .status()
                .expect("Failed to run CPUEmulator")
                .success(),
            "Bad status when running CPUEmulator"
        );
    }
}

fn vm_translate(asm_file: &Path) {
    let module_id = asm_file
        .file_stem()
        .unwrap_or_else(|| panic!("Expected *.asm file, got `{}`", asm_file.display()))
        .to_str()
        .expect("Filename has to be unicode.");
    let result_filename = asm_file.with_extension("asm");
    let asm_file = fs::read_to_string(asm_file)
        .unwrap_or_else(|_| panic!("Couldn't read {}.", asm_file.display()));

    let mut jmp_idx = 0;
    let mut result = trimmed_lines(&asm_file)
        .map(|l| {
            let o = l
                .parse()
                .unwrap_or_else(|e| panic!("Error parsing `{l}`: {}", e));
            let asm = match o {
                VmCommand::Add => pop_d() + "\n" + &peek() + "\nM=M+D",
                VmCommand::Sub => pop_d() + "\n" + &peek() + "\nM=M-D",
                VmCommand::Neg => peek() + "\nM=-M",
                VmCommand::Eq => compare_command("JEQ", &mut jmp_idx),
                VmCommand::Gt => compare_command("JGT", &mut jmp_idx),
                VmCommand::Lt => compare_command("JLT", &mut jmp_idx),
                VmCommand::And => pop_d() + "\n" + &peek() + "\nM=M&D",
                VmCommand::Or => pop_d() + "\n" + &peek() + "\nM=M|D",
                VmCommand::Not => peek() + "\nM=!M",
                VmCommand::Push(k) => k.push(module_id),
                VmCommand::Pop(k) => k.pop(module_id),
            };
            format!("// {l}\n{asm}")
        })
        .collect::<Vec<String>>()
        .join("\n");

    result.push('\n');
    fs::write(result_filename, result).expect("Failed writing assembly file.");
}

impl MemoryLocation {
    fn push(&self, module_id: &str) -> String {
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

    fn pop(&self, module_id: &str) -> String {
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

enum VmCommand {
    Add,
    Sub,
    Neg,
    Eq,
    Gt,
    Lt,
    And,
    Or,
    Not,
    Push(MemoryLocation),
    Pop(MemoryLocation),
}

enum MemoryLocation {
    Constant(usize),
    Local(usize),
    Argument(usize),
    This(usize),
    That(usize),
    Temp(usize),
    Pointer(usize),
    Static(usize),
}

fn pop_from(a_expr: impl Display) -> String {
    format!("{}\n@{a_expr}\nM=D", pop_d())
}

fn push_to(a_expr: impl Display) -> String {
    format!("@{a_expr}\nD=M\n{}", push_d())
}

fn compare_command(cmp: &str, jmp_idx: &mut i32) -> String {
    *jmp_idx += 1;
    format!(
        r#"{pop_d}
{peek}
D=M-D
M=-1
@TRUE{jmp_idx}
D;{cmp}
@SP
A=M-1
M=0
(TRUE{jmp_idx})"#,
        pop_d = pop_d(),
        peek = peek()
    )
}

fn push_from_addr(p_name: &str, offset: usize) -> String {
    format!(
        // TODO: optimize for offset=0 and offset=1
        r#"@{offset}
D=A
@{p_name}
A=M+D
D=M
{push_d}"#,
        push_d = push_d()
    )
}

fn pop_to_addr(p_name: &str, offset: usize) -> String {
    format!(
        // TODO: optimize for offset=0 and offset=1
        r#"@{offset}
D=A
@{p_name}
D=M+D
@R13
M=D
@SP
AM=M-1
D=M
@R13
A=M
M=D"#
    )
}

/// Push D on the stack
fn push_d() -> String {
    "@SP\nM=M+1\nA=M-1\nM=D".to_owned()
}

/// Pop one element from the stack to D
fn pop_d() -> String {
    "@SP\nAM=M-1\nD=M".to_owned()
}

/// Set A to the last element on the stack
fn peek() -> String {
    "@SP\nA=M-1".to_owned()
}

fn trimmed_lines(s: &str) -> impl Iterator<Item = &str> {
    s.lines()
        .map(|l| strip_comment(l).trim())
        .filter(|l| !l.is_empty())
}

fn strip_comment(s: &str) -> &str {
    if let Some((content, _comment)) = s.split_once("//") {
        content
    } else {
        s
    }
}
impl FromStr for VmCommand {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split_whitespace();
        let operation = parts.next().expect("Cannot parse empty string");
        let operation = match operation {
            "add" => VmCommand::Add,
            "sub" => VmCommand::Sub,
            "neg" => VmCommand::Neg,
            "eq" => VmCommand::Eq,
            "gt" => VmCommand::Gt,
            "lt" => VmCommand::Lt,
            "and" => VmCommand::And,
            "or" => VmCommand::Or,
            "not" => VmCommand::Not,
            "push" => VmCommand::Push(MemoryLocation::from(&mut parts)?),
            "pop" => VmCommand::Pop(MemoryLocation::from(&mut parts)?),
            _ => return Err("Unexpected expression"),
        };
        if let Some(_) = parts.next() {
            return Err("Spurious element after command");
        }
        Ok(operation)
    }
}

impl MemoryLocation {
    fn from<'a>(parts: &mut impl Iterator<Item = &'a str>) -> Result<Self, &'static str> {
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
