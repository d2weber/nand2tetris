mod asm_generators;
mod memory_location;

use asm_generators::*;
use memory_location::MemoryLocation;

use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
    str::FromStr,
};

fn main() {
    let mut args = env::args();
    if args.len() != 2 {
        panic!("Expect single parameter to `*.vm` file.");
    }
    let filename = args.next_back().unwrap();
    compile_path(Path::new(&filename)).unwrap();
}

#[cfg(test)]
mod test {
    use std::process::{Command, Stdio};

    use super::*;

    mod project7 {
        use super::*;
        #[test]
        fn simple_add() {
            test_file("../../7/StackArithmetic/SimpleAdd/SimpleAdd.vm");
        }

        #[test]
        fn stack_test() {
            test_file("../../7/StackArithmetic/StackTest/StackTest.vm");
        }

        #[test]
        fn basic_test() {
            test_file("../../7/MemoryAccess/BasicTest/BasicTest.vm");
        }

        #[test]
        fn pointer_test() {
            test_file("../../7/MemoryAccess/PointerTest/PointerTest.vm");
        }

        #[test]
        fn static_test() {
            test_file("../../7/MemoryAccess/StaticTest/StaticTest.vm");
        }
    }

    #[test]
    fn basic_loop() {
        test_file("../ProgramFlow/BasicLoop/BasicLoop.vm")
    }

    #[test]
    fn fibonacci_series() {
        test_file("../ProgramFlow/FibonacciSeries/FibonacciSeries.vm")
    }

    #[test]
    fn simple_function() {
        test_file("../FunctionCalls/SimpleFunction/SimpleFunction.vm")
    }

    #[test]
    fn fibonacci_element() {
        test_dir("../FunctionCalls/FibonacciElement/");
    }

    #[test]
    fn nested_call() {
        test_dir("../FunctionCalls/NestedCall/");
    }

    fn test_file(vm_file: &str) {
        let vm_file = Path::new(vm_file);
        test_path(vm_file, &vm_file.with_extension("tst"));
    }

    fn test_dir(dir: &str) {
        let dir = Path::new(dir);
        let dir_name = dir.file_name().unwrap();
        test_path(dir, &dir.join(dir_name).with_extension("tst"));
    }

    /// Compile provided vm file to asm, and check result with a `*.tst` file
    fn test_path(path: &Path, tst_file_path: &Path) {
        let cargo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let path = cargo_root.join(path);
        compile_path(&path).unwrap();
        assert!(
            Command::new("bash")
                .arg("../../../tools/CPUEmulator.sh")
                .arg(tst_file_path)
                .current_dir(cargo_root)
                .stdout(Stdio::null())
                .status()
                .expect("Failed to run CPUEmulator")
                .success(),
            "Bad status when running CPUEmulator"
        );
    }
}

fn compile_path(path: &Path) -> std::io::Result<()> {
    if path.is_file() {
        let mut out = BufWriter::new(File::open(path.with_extension("asm"))?);
        compile_file(path, &mut out);
        Ok(())
    } else if path.is_dir() {
        let name = path
            .file_name()
            .expect("Already checked that it's a directory");
        let asm_file = path.join(name).with_extension("asm");
        let mut out = BufWriter::new(File::create(asm_file)?);
        // TODO: Error when no vm file is found
        for dir_entry in fs::read_dir(path)? {
            let file = dir_entry?.path();
            if file.extension().is_some_and(|e| e == "vm") {
                compile_file(&file, &mut out)
            }
        }
        Ok(())
    } else {
        Err(std::io::ErrorKind::NotFound.into())
    }
}

fn compile_file(vm_file: &Path, out: &mut impl Write) {
    let module_id = vm_file
        .file_stem()
        .unwrap_or_else(|| panic!("Expected *.vm file, got `{}`", vm_file.display()))
        .to_str()
        .expect("Filename has to be unicode.");
    let asm_file = fs::read_to_string(vm_file)
        .unwrap_or_else(|_| panic!("Couldn't read {}.", vm_file.display()));

    out.write_all(
        format!(
            "@256\nD=A\n@SP\nM=D\n{}\n0;JMP\n",
            call_asm(&"Sys.init", 0, "Sys.init$ret.0")
        )
        .as_bytes(),
    )
    .unwrap_or_else(|_| panic!("Error writing {module_id}"));
    let mut jmp_idx = 0;
    let mut return_function_idx = 0..;
    let mut current_function = "root".to_owned();
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
                VmCommand::Label(label) => format!("({current_function}${label})"),
                VmCommand::Goto(label) => format!("@{current_function}${label}\n0;JMP"),
                VmCommand::IfGoto(label) => {
                    pop_d() + &format!("\n@{current_function}${label}\nD;JNE")
                }
                VmCommand::Function(name, nvars) => {
                    current_function = name;
                    format!("({current_function}){}", zero_local(nvars))
                }
                VmCommand::Return => return_asm(),
                VmCommand::Call(name, nargs) => {
                    let idx = return_function_idx.next().unwrap();
                    let return_label = format!("{current_function}$ret.{idx}");
                    call_asm(&name, nargs, &return_label)
                }
            };
            format!("// {l}\n{asm}")
        })
        .collect::<Vec<String>>()
        .join("\n");

    result.push('\n');
    out.write_all(result.as_bytes())
        .expect("Failed to write output file");
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
    Label(String),
    Goto(String),
    IfGoto(String),
    Function(String, usize),
    Return,
    Call(String, usize),
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
            "label" => VmCommand::Label(parts.next().ok_or("Missing label name")?.to_owned()),
            "goto" => VmCommand::Goto(parts.next().ok_or("Missing goto label")?.to_owned()),
            "if-goto" => VmCommand::IfGoto(parts.next().ok_or("Missing if-goto label")?.to_owned()),
            "function" => VmCommand::Function(
                parts.next().ok_or("Missing function name")?.to_owned(),
                parts
                    .next()
                    .ok_or("Missing nargs for function")?
                    .parse()
                    .map_err(|_| "Unable to parse nargs")?,
            ),
            "return" => VmCommand::Return,
            "call" => VmCommand::Call(
                parts.next().ok_or("Missing function name")?.to_owned(),
                parts
                    .next()
                    .ok_or("Missing nargs for function")?
                    .parse()
                    .map_err(|_| "Unable to parse nargs")?,
            ),
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
