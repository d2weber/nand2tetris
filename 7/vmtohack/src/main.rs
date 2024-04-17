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
            if let Some(l) = l.strip_prefix("push ") {
                if let Some(number) = l.strip_prefix("constant ") {
                    let number: i32 = number.parse().unwrap_or_else(|_| {
                        panic!(
                            "Got `{number}`, but expected number literal when pushing a constant."
                        )
                    });

                    format!("@{number}\nD=A\n{}", push_d("SP"))
                } else {
                    todo!()
                }
            } else {
                match l {
                    "add" => pop_and_peek("SP") + "\nM=M+D",
                    "sub" => pop_and_peek("SP") + "\nM=M-D",
                    "neg" => peek("SP") + "\nM=-M",
                    "eq" => compare_command("JEQ", &mut jmp_idx),
                    "gt" => compare_command("JGT", &mut jmp_idx),
                    "lt" => compare_command("JLT", &mut jmp_idx),
                    "and" => pop_and_peek("SP") + "\nM=M&D",
                    "or" => pop_and_peek("SP") + "\nM=M|D",
                    "not" => peek("SP") + "\nM=!M",
                    _ => panic!("Unexpected expression `{l}`"),
                }
            }
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
