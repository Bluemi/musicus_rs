mod musicus;
mod file_manager;

use pancurses::{initscr, endwin};

fn main() {
    let window = initscr();
	let win = window.subwin(20, 20, 10, 10).unwrap();
	win.border('|', '|', '-', '-', '+', '+', '+', '+');
	win.mvprintw(1, 1, "hey");
	window.mvprintw(1, 1, "Hello world!");
	window.refresh();
	window.getch();
	endwin();
}
