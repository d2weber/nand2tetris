//; This file is part of www.nand2tetris.org
//; and the book "The Elements of Computing Systems"
//; by Nisan and Schocken, MIT Press.
//; File name: projects/4/Fill.asm

//; Runs an infinite loop that listens to the keyboard input.
//; When a key is pressed (any key), the program blackens the screen,
//; i.e. writes "black" in every pixel. When no key is pressed,
//; the screen should be cleared.

(MAINLOOP)

//; pattern=white
@pattern
M=0

//; If KBD==0 skip to fill with white pattern
@KBD
D=M
@FILL
D;JEQ

//; pattern=black
@pattern
M=-1

//; Fill screen with pattern
(FILL)

//; let p point to last pixel on screen
@SCREEN
D=A
@8191
D=D+A
@p
M=D

(FILL_LOOP)

//; put pattern to RAM[p]
@pattern
D=M
@p
A=M
M=D

//; --p; if p==SCREEN break;
@p
D=M
M=M-1
@SCREEN
D=A-D
@FILL_LOOP
D;JNE

@MAINLOOP
0;JMP
