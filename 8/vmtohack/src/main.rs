mod asm_generators;
mod memory_location;

use asm_generators::*;
use memory_location::MemoryLocation;

use std::{env, fs, path::Path, str::FromStr};

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

    mod project7 {
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
    }

    #[test]
    fn basic_loop() {
        check_tst("../ProgramFlow/BasicLoop/BasicLoop.vm")
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
