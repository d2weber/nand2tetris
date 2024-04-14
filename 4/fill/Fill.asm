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

@8192
D=A
@counter //; counts backwards
M=D

//; Start address to fill
@SCREEN
D=A
@p
M=D

(FILLLOOP)

//; if counter==0 goto MAINLOOP; --counter
@counter
D=M
M=M-1
@MAINLOOP
D;JEQ

//; put pattern to current address
@pattern
D=M
@p
A=M
M=D
@p
M=M+1

@FILLLOOP
0;JMP
