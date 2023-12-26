#![feature(try_blocks)]
#![feature(let_chains)]
#![feature(linked_list_remove)]

use crate::interpreter::wav::SampleSize;

mod syntax;
mod take;
mod compiler;
mod interpreter;


const EXAMPLE_PROGRAM: &str = include_str!("../test.musical");


fn test_value(s: &str) -> String {
    let mut token_stream = syntax::lexer::TokenStream::from(s.chars());
    let value = syntax::parser::Value::try_from(&mut token_stream);

    match value {
        Ok(v) => format!("{v}"),
        Err(err) => format!("{err:?}"),
    }
}


fn main() {
    syntax::lexer::TokenStream::from(EXAMPLE_PROGRAM.chars()).for_each(|token| println!("{token}"));

    println!("--------------------------------------------");

    let script = syntax::parser::Script::try_from(EXAMPLE_PROGRAM).expect("Error");
    println!("{script}");

    println!("--------------------------------------------");

    let program = compiler::Program::try_from(&script).expect("Error");
    println!("{program}");

    std::fs::write("test.wav", interpreter::wav::interpret(&program, 48000, SampleSize::Large)).expect("uga buga");
}
