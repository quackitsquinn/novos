use core::f32::consts::E;

use alloc::{string::String, vec::Vec};
use core::fmt::Write;
use rhai::{Dynamic, Engine, ParseErrorType};

use crate::{
    display::terminal::{self, Terminal},
    interrupts::{
        self,
        hardware::keyboard::{KeyboardDriver, KEYBOARD},
    },
    print, println, terminal,
};

struct Context {
    rhai: rhai::Engine,
    newline: bool,
    lines: Vec<String>,
}

pub fn run() {
    interrupts::enable();
    let mut context = Context {
        rhai: create_engine(),
        newline: true,
        lines: Vec::new(),
    };
    loop {
        let mut keyboard = KEYBOARD.lock();
        update_display(&mut context, &mut keyboard);
        update_rhai(&mut keyboard, &mut context)
    }
}

fn update_display(context: &mut Context, driver: &mut KeyboardDriver) {
    if context.newline {
        let mut terminal = terminal!();
        prompt(&mut terminal, context);
        context.newline = false;
    }
    if driver.has_new_input() {
        let mut terminal = terminal!();
        for _ in 0..driver.backspaces() {
            // TODO: backspace
        }
        let new_input = driver.read_new_input();
        if new_input.contains("\n") {
            context.newline = true;
        }

        terminal.push_str(&new_input);
    }
}

fn create_engine() -> Engine {
    let mut engine = Engine::new();
    // Register functions, variables, etc. here
    fn println(s: &str) {
        println!("{}", s);
    }

    fn print(s: &str) {
        print!("{}", s);
    }

    engine.register_fn("print", print);
    engine.register_fn("println", println);
    engine.on_print(|s| print!("{}", s));

    engine
}

fn update_rhai(driver: &mut KeyboardDriver, context: &mut Context) {
    let line = driver.read_line();

    if line.is_none() {
        return;
    }

    let line = line.unwrap();

    let mut full = context.lines.join("\n");

    write!(full, "\n{}", line).expect(":(");

    let rhai = &mut context.rhai;

    let ast = rhai.compile(&line);

    if let Err(e) = ast {
        if *e.err_type() == ParseErrorType::UnexpectedEOF {
            // Need more, push to lines
            context.lines.push(line);
            return;
        }

        print!("Compilation error! {}", e);
        return;
    }

    let ast = ast.unwrap();

    let res = rhai.eval_ast::<Dynamic>(&ast);

    if let Err(e) = res {
        println!("Error! {}", e);
        return;
    }

    println!("=> {}", res.unwrap())
}

fn prompt(terminal: &mut Terminal, context: &mut Context) {
    terminal.push_str(">> ");
}
