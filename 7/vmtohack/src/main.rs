use std::{env, fs, path::Path};

fn main() {
    let mut args = env::args();
    if args.len() != 2 {
        panic!("Expect single parameter to `*.vm` file.");
    }
    let filename = args.next_back().unwrap();
    let result = vm_translate(&Path::new(&filename));

    // I prefer to print to stdout, users can easily pipe to a file
    print!("{result}");
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn simple_add() {
        let result = vm_translate(&Path::new("../StackArithmetic/SimpleAdd/SimpleAdd.vm"));
        assert_eq!(
            result,
            concat!(
                "@7\nD=A\n@SP\nM=M+1\nA=M-1\nM=D\n",
                "@8\nD=A\n@SP\nM=M+1\nA=M-1\nM=D\n",
                "@SP\nAM=M-1\nD=M\n@SP\nA=M-1\nM=M+D\n"
            )
        );
    }

    #[test]
    fn stack_test() {
        let result = vm_translate(&Path::new("../StackArithmetic/StackTest/StackTest.vm"));
        assert_eq!(
            result,
            concat!(
                "@7\nD=A\n@SP\nM=M+1\nA=M-1\nM=D\n",
                "@8\nD=A\n@SP\nM=M+1\nA=M-1\nM=D\n",
                "@SP\nAM=M-1\nD=M\n@SP\nA=M-1\nM=M+D\n"
            )
        );
    }
}

fn vm_translate(asm_file: &Path) -> String {
    let asm_file =
        fs::read_to_string(asm_file).expect(&format!("Couldn't read {}.", asm_file.display()));

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

                    format!(
                        "\
                        @{number}\n\
                        D=A\n\
                        @SP\n\
                        M=M+1\n\
                        A=M-1\n\
                        M=D"
                    )
                } else {
                    todo!()
                }
            } else {
                match l {
                    "add" => format!("{POP_AND_PEEK}\nM=M+D"),
                    "sub" => format!("{POP_AND_PEEK}\nM=M-D"),
                    "neg" => format!("{PEEK}\nM=-M"),
                    "eq" => compare_command("JEQ", &mut jmp_idx),
                    "gt" => compare_command("JGT", &mut jmp_idx),
                    "lt" => compare_command("JLT", &mut jmp_idx),
                    "and" => format!("{POP_AND_PEEK}\nM=M&D"),
                    "or" => format!("{POP_AND_PEEK}\nM=M|D"),
                    "not" => format!("{PEEK}\nM=!M"),
                    _ => panic!("Unexpected expression `{l}`"),
                }
            }
        })
        .collect::<Vec<String>>()
        .join("\n");

    result.push('\n');
    result
}

fn compare_command(cmp: &str, jmp_idx: &mut i32) -> String {
    *jmp_idx += 1;
    format!(
        r#"{POP_AND_PEEK}
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

const PEEK: &str = "@SP\nA=M-1";

const POP_AND_PEEK: &str = r#"@SP
AM=M-1
D=M
@SP
A=M-1"#;

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
