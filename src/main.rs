use std::{
    io::{self, Write},
    rc::Rc,
};

pub mod lex;
mod port;
use port::{current_output_port, Port};

#[derive(Clone)]
pub enum Object {
    Boolean(bool),
    Char(char),
    Null,
    Pair(Rc<(Object, Object)>),
    Procedure(/* TODO */),
    Symbol(Rc<std::string::String>),
    Bytevector(Rc<Vec<u8>>),
    EofObject,
    Exact(u64),
    Inexact(f64),
    Port(Port),
    String(Rc<Vec<char>>),
    Vector(Rc<Vec<Object>>),
    Record(/* TODO */),
}
use Object::*;

fn cons(car: Object, cdr: Object) -> Object {
    Pair(Rc::new((car, cdr)))
}

fn write_simple1(obj: Object) -> Object {
    write_simple2(obj, current_output_port())
}

fn write_simple2(obj: Object, port: Object) -> Object {
    if let Port(mut p) = port {
        write_impl(&obj, &mut p).unwrap();
        Object::Null
    } else {
        panic!("2nd arg to write-simple must be a port.")
    }
}

fn write_impl(obj: &Object, p: &mut Port) -> Result<(), io::Error> {
    match obj {
        Boolean(true) => write!(p, "#t")?,
        Boolean(false) => write!(p, "#f")?,
        Char('\x07') => write!(p, r"#\alarm")?,
        Char('\x08') => write!(p, r"#\backspace")?,
        Char('\x7F') => write!(p, r"#\delete")?,
        Char('\x1B') => write!(p, r"#\escape")?,
        Char('\n') => write!(p, r"#\newline")?,
        Char('\0') => write!(p, r"#\null")?,
        Char('\r') => write!(p, r"#\return")?,
        Char(' ') => write!(p, r"#\space")?,
        Char('\t') => write!(p, r"#\tab")?,
        Char(c) => write!(p, r"#\{c}")?,
        Null => write!(p, "()")?,
        Pair(rc) => {
            write!(p, "(")?;
            write_impl(&rc.0, p)?;
            write_cdr(&rc.1, p)?;
            write!(p, ")")?;
        }
        Procedure() => write!(p, "<procedure>")?,
        Symbol(s) => write!(p, "{}", s)?,
        Bytevector(v) => {
            write!(p, "#u8(")?;
            if v.len() > 0 {
                write!(p, "{}", v[0])?;
                for b in &v[1..] {
                    write!(p, " {}", b)?;
                }
            }
            write!(p, ")")?;
        }
        EofObject => write!(p, "<eof>")?,
        Exact(i) => write!(p, "{}", i)?,
        Inexact(f) => write!(p, "{}", f)?,
        Port(_) => write!(p, "<port>")?,
        String(s) => {
            for c in s.iter() {
                write!(p, "{}", c)?;
            }
        }
        Vector(v) => {
            write!(p, "#(")?;
            if v.len() > 0 {
                write_impl(&v[0], p)?;
                for x in &v[1..] {
                    write_impl(x, p)?;
                }
            }
            write!(p, ")")?;
        }
        Record() => write!(p, "<record>")?,
    };
    Ok(())
}

fn write_cdr(cdr: &Object, p: &mut Port) -> Result<(), io::Error> {
    match cdr {
        Null => {}
        Pair(rc) => {
            write!(p, " ")?;
            write_impl(&rc.0, p)?;
            write_cdr(&rc.1, p)?;
        }
        _ => {
            write!(p, " . ")?;
            write_impl(cdr, p)?;
        }
    };
    Ok(())
}

fn main() {

    println!("mibph!");
    println!("eggs? ;_;");
    println!();
    println!(
        "objects are {} bytes in memory.",
        std::mem::size_of::<Object>()
    );

    println!("here is an improper list:");
    write_simple1(cons(Exact(5), cons(cons(Exact(7), Null), Exact(9))));
    println!();
    println!();
    println!("ask me to lex some tokens for you:");

    for s in std::io::stdin().lines() {
        let s = &s.unwrap();
        match lex::token(s) {
            Ok(("", t)) => println!("that is a {t:?} token."),
            Ok((r, t)) => println!("that is {t:?} token and some extra stuff (\"{r}\")."),
            Err(e) => println!("that is not a token! {e}"),
        };
    }
}
