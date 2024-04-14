//; This file is part of www.nand2tetris.org
//; and the book "The Elements of Computing Systems"
//; by Nisan and Schocken, MIT Press.
//; File name: projects/4/Fill.asm

//; Runs an infinite loop that listens to the keyboard input.
//; When a key is pressed (any key), the program blackens the screen,
//; i.e. writes "black" in every pixel. When no key is pressed,
//; the screen should be cleared.

(MAIN_LOOP)

//; set counter to end
@8192
D=A
@counter
M=D

//; if KBD==0 goto FILL_WHITE else goto FILL_BLACK
@KBD
D=M
//; Use separate implementations for black and white to save
//; the instruction for color loading in the busy loop
//; This does not really speed up things in the emulator but
//; it probably would on real hardware
@FILL_WHITE
D;JEQ
@FILL_BLACK
0;JMP

(FILL_WHITE)
@counter
M=M-1   //; --counter
D=M
@SCREEN
A=D+A   //; set RAM[SCREEN+counter]
M=0

//; repeat if D!=0 else goto MAIN_LOOP
@FILL_WHITE
D;JNE
@MAIN_LOOP
0;JMP

(FILL_BLACK)
@counter
M=M-1   //; --counter
D=M
@SCREEN
A=D+A   //; set RAM[SCREEN+counter]
M=-1

//; repeat if D!=0 else goto MAIN_LOOP
@FILL_BLACK
D;JNE
@MAIN_LOOP
0;JMP
