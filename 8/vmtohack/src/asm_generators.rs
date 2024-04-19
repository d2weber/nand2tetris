use std::fmt::Display;

pub fn pop_from(a_expr: impl Display) -> String {
    format!("{}\n@{a_expr}\nM=D", pop_d())
}

pub fn push_to(a_expr: impl Display) -> String {
    format!("@{a_expr}\nD=M\n{}", push_d())
}

pub fn compare_command(cmp: &str, jmp_idx: &mut i32) -> String {
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

pub fn push_from_addr(p_name: &str, offset: usize) -> String {
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

pub fn pop_to_addr(p_name: &str, offset: usize) -> String {
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
pub fn push_d() -> String {
    "@SP\nM=M+1\nA=M-1\nM=D".to_owned()
}

/// Pop one element from the stack to D
pub fn pop_d() -> String {
    "@SP\nAM=M-1\nD=M".to_owned()
}

/// Set A to the last element on the stack
pub fn peek() -> String {
    "@SP\nA=M-1".to_owned()
}

pub fn zero_local(nvars: usize) -> String {
    let mut result = String::new();
    if nvars == 0 {
        return result;
    }
    result += "@LCL\nA=M\nM=0";
    for _ in 1..nvars {
        result += "\nA=A+1\nM=0"
    }
    result
}

pub fn return_asm() -> String {
    pop_d()
        + r#"
@ARG // Return value
A=M
M=D
@LCL // Store LCL in R13
D=M
@R13
M=D
@5 // Store retAddr in R12
A=D-A
D=M
@R12
M=D
@ARG // Reposition SP
D=M+1
@SP
M=D
@R13 // Decrement
AM=M-1
D=M
@THAT
M=D
@R13 // Decrement
AM=M-1
D=M
@THIS
M=D
@R13 // Decrement
AM=M-1
D=M
@ARG
M=D
@R13 // Decrement
AM=M-1
D=M
@LCL
M=D
@R12 // get retAddr
A=M
0;JMP"#
}
