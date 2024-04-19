use std::{env, fmt::Display, fs, path::Path};

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
            let mut parts = l.split_whitespace();
            let operation = parts.next().expect("Empty lines have been filtered out");
            let asm = match operation {
                "add" => pop_d() + "\n" + &peek() + "\nM=M+D",
                "sub" => pop_d() + "\n" + &peek() + "\nM=M-D",
                "neg" => peek() + "\nM=-M",
                "eq" => compare_command("JEQ", &mut jmp_idx),
                "gt" => compare_command("JGT", &mut jmp_idx),
                "lt" => compare_command("JLT", &mut jmp_idx),
                "and" => pop_d() + "\n" + &peek() + "\nM=M&D",
                "or" => pop_d() + "\n" + &peek() + "\nM=M|D",
                "not" => peek() + "\nM=!M",
                "push" | "pop" => {
                    let kind = parts
                        .next()
                        .unwrap_or_else(|| panic!("Missing kind after {operation}: `{l}`"));
                    let offset: usize = parts
                        .next()
                        .unwrap_or_else(|| panic!("Missing number after {operation}: `{l}`"))
                        .parse()
                        .unwrap_or_else(|_| panic!("Could not parse number in `{l}`"));
                    push_or_pop(operation, kind, offset, module_id)
                }
                _ => panic!("Unexpected expression `{l}`"),
            };
            format!("// {l}\n{asm}")
        })
        .collect::<Vec<String>>()
        .join("\n");

    result.push('\n');
    fs::write(result_filename, result).expect("Failed writing assembly file.");
}

fn push_or_pop(operation: &str, kind: &str, offset: usize, module_id: &str) -> String {
    match (operation, kind) {
        ("push", "constant") => format!("@{offset}\nD=A\n{}", push_d()),
        ("push", "local") => push_from_addr("LCL", offset),
        ("push", "argument") => push_from_addr("ARG", offset),
        ("push", "this") => push_from_addr("THIS", offset),
        ("push", "that") => push_from_addr("THAT", offset),
        ("push", "temp") => push_to(5 + offset),
        ("pop", "local") => pop_to_addr("LCL", offset),
        ("pop", "argument") => pop_to_addr("ARG", offset),
        ("pop", "this") => pop_to_addr("THIS", offset),
        ("pop", "that") => pop_to_addr("THAT", offset),
        ("pop", "temp") => pop_from(5 + offset),
        ("push", "pointer") => match offset {
            0 => push_to("THIS"),
            1 => push_to("THAT"),
            _ => panic!("Cannot {operation} to pointer `{offset}`"),
        },
        ("pop", "pointer") => match offset {
            0 => pop_from("THIS"),
            1 => pop_from("THAT"),
            _ => panic!("Cannot {operation} to pointer `{offset}`"),
        },
        ("push", "static") => push_to(format!("{module_id}.{offset}")),
        ("pop", "static") => pop_from(format!("{module_id}.{offset}")),
        _ => panic!("Cannot {operation} from `{kind}`"),
    }
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
