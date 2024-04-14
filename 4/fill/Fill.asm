//; This file is part of www.nand2tetris.org
//; and the book "The Elements of Computing Systems"
//; by Nisan and Schocken, MIT Press.
//; File name: projects/4/Fill.asm

//; Runs an infinite loop that listens to the keyboard input.
//; When a key is pressed (any key), the program blackens the screen,
//; i.e. writes "black" in every pixel. When no key is pressed,
//; the screen should be cleared.

(MAIN_LOOP)

//; let p point to last pixel on screen
@SCREEN
D=A
@8192
D=D+A
@p
M=D

//; if KBD==0 goto FILL_WHITE else goto FILL_BLACK
@KBD
D=M
@FILL_WHITE
D;JEQ
@FILL_BLACK
0;JMP

(FILL_WHITE)
@p
M=M-1 //; --p
D=M   //; save p for later
A=M   //; set RAM[p]
M=0

//; repeat if D!=SCREEN else goto MAIN_LOOP; --p;
@SCREEN
D=A-D
@FILL_WHITE
D;JNE
@MAIN_LOOP
0;JMP

(FILL_BLACK)
@p
M=M-1 //; --p
D=M   //; save p for later
A=M   //; set RAM[p]
M=-1

//; repeat if D!=SCREEN else goto MAIN_LOOP; --p;
@SCREEN
D=A-D
@FILL_BLACK
D;JNE
@MAIN_LOOP
0;JMP
