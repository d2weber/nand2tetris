use std::{env, fs, path::Path};

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
        check_tst("../StackArithmetic/SimpleAdd/SimpleAdd.vm");
    }

    #[test]
    fn stack_test() {
        check_tst("../StackArithmetic/StackTest/StackTest.vm");
    }

    #[test]
    fn basic_test() {
        check_tst("../MemoryAccess/BasicTest/BasicTest.vm");
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
    let result_filename = asm_file.with_extension("asm");
    let asm_file = fs::read_to_string(asm_file)
        .unwrap_or_else(|_| panic!("Couldn't read {}.", asm_file.display()));

    let mut jmp_idx = 0;
    let mut result = trimmed_lines(&asm_file)
        .map(|l| {
            let mut parts = l.split_whitespace();
            let operation = parts.next().expect("Empty lines have been filtered out");
            let asm = match operation {
                "add" => pop_and_peek("SP") + "\nM=M+D",
                "sub" => pop_and_peek("SP") + "\nM=M-D",
                "neg" => peek("SP") + "\nM=-M",
                "eq" => compare_command("JEQ", &mut jmp_idx),
                "gt" => compare_command("JGT", &mut jmp_idx),
                "lt" => compare_command("JLT", &mut jmp_idx),
                "and" => pop_and_peek("SP") + "\nM=M&D",
                "or" => pop_and_peek("SP") + "\nM=M|D",
                "not" => peek("SP") + "\nM=!M",
                "push" | "pop" => {
                    let namespace = parts
                        .next()
                        .unwrap_or_else(|| panic!("Missing kind after pop: `{l}`"));
                    let offset: usize = parts
                        .next()
                        .unwrap_or_else(|| panic!("Missing number after pop: `{l}`"))
                        .parse()
                        .unwrap_or_else(|_| panic!("Could not parse number in `{l}`"));
                    match (operation, namespace) {
                        ("push", "constant") => format!("@{offset}\nD=A\n{}", push_d("SP")),
                        ("push", "local") => read_to_d("LCL", offset) + "\n" + &push_d("SP"),
                        ("push", "argument") => read_to_d("ARG", offset) + "\n" + &push_d("SP"),
                        ("push", "this") => read_to_d("THIS", offset) + "\n" + &push_d("SP"),
                        ("push", "that") => read_to_d("THAT", offset) + "\n" + &push_d("SP"),
                        ("push", "temp") => read_tmp(offset) + "\n" + &push_d("SP"),
                        ("pop", "local") => write_from_d("LCL", offset),
                        ("pop", "argument") => write_from_d("ARG", offset),
                        ("pop", "this") => write_from_d("THIS", offset),
                        ("pop", "that") => write_from_d("THAT", offset),
                        ("pop", "temp") => write_tmp(offset),
                        _ => panic!("Cannot {operation} from `{namespace}`"),
                    }
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

fn compare_command(cmp: &str, jmp_idx: &mut i32) -> String {
    *jmp_idx += 1;
    pop_and_peek("SP")
        + &format!(
            r#"
D=M-D
M=-1
@TRUE{jmp_idx}
D;{cmp}
@SP
A=M-1
M=0
(TRUE{jmp_idx})"#
        )
}

fn read_tmp(mut offset: usize) -> String {
    offset += 5;
    format!("@{offset}\nD=M")
}

fn read_to_d(p_name: &str, offset: usize) -> String {
    format!(
        // TODO: optimize for offset=0 and offset=1
        r#"@{offset}
D=A
@{p_name}
A=M+D
D=M"#
    )
}

fn write_tmp(mut offset: usize) -> String {
    offset += 5;
    let pop = pop_to_d("SP");
    format!("{pop}\n@{offset}\nM=D")
}

fn write_from_d(p_name: &str, offset: usize) -> String {
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

fn push_d(p_name: &str) -> String {
    format!("@{p_name}\nM=M+1\nA=M-1\nM=D")
}

/// Decrease stack pointer and set A to the popped element
fn pop(p_name: &str) -> String {
    format!("@{p_name}\nAM=M-1")
}

/// Pop one element of the stack into D
fn pop_to_d(p_name: &str) -> String {
    format!("{}\nD=M", pop(p_name))
}

/// Set A to the last element on the stack
fn peek(p_name: &str) -> String {
    format!("@{p_name}\nA=M-1")
}

/// Pop one element of the stack into D and peek
fn pop_and_peek(p_name: &str) -> String {
    format!("{}\n{}", pop_to_d(p_name), peek(p_name))
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
