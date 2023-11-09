use std::collections::HashMap;

use super::{
    net::{Net, NetBuilder},
    term::TermPtr,
    var::VarUse,
};
use chumsky::{extra::State, prelude::*, text::keyword, Parser};

// <book> ::= (<def> (';' <def>)* ';')?
// <def> ::= 'def' <ident> '(' <terms> ')' '=' <eqns>
// <terms> ::= '(' <term> (',' <term>)* ')'
// <term> ::= <var> | era | ctr | dup
// <eqns> ::= (<eqn> ('&' <eqn>)*)?
// <eqn> ::= <term> '~' <term>
// <var> ::= <ident>
// <era> ::= '*'
// <ctr> ::= (<term> <term>)
// <dup> ::= [<term> <term>]

pub fn parse(src: &str, net: &mut Net) -> bool {
    let mut state = ParserState::new(net);
    match parse_book()
        .parse_with_state(src.trim(), &mut state)
        .into_result()
    {
        Ok(_) => true,
        Err(_) => false,
    }
}
// let src = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();

struct ParserState<'a> {
    net: &'a mut Net,
    vars: HashMap<&'a str, VarUse>,
    defs: HashMap<&'a str, u32>,
}
impl<'a> ParserState<'a> {
    pub fn new(net: &'a mut Net) -> Self {
        Self {
            net,
            vars: Default::default(),
            defs: Default::default(),
        }
    }
}

fn parse_term<'a>() -> impl Parser<'a, &'a str, TermPtr, State<ParserState<'a>>> {
    return recursive::<'a, &'a str, TermPtr, State<ParserState<'a>>, _, _>(|term| {
        let era =
            just('*')
                .ignored()
                .padded()
                .map_with_state(|_, _, state: &mut ParserState<'a>| {
                    //
                    state.net.era()
                });

        let var = text::ident()
            .padded()
            .map_with_state(|name, _, state: &mut ParserState| {
                if let Some(var_use) = state.vars.remove(name) {
                    return TermPtr::Ptr(var_use.ptr());
                } else {
                    let var = state.net.var();
                    state.vars.insert(name, var.0);
                    TermPtr::Ptr(var.1.ptr())
                }
            });

        let dup = term
            .clone()
            .then(term.clone())
            .delimited_by(just('[').padded(), just(']').padded())
            .map_with_state(|(left, right), _, state: &mut ParserState<'a>| {
                state.net.app(left, right)
            });
        let ctr = term
            .clone()
            .then(term.clone())
            .delimited_by(just('(').padded(), just(')').padded())
            .map_with_state(|(left, right), _, state: &mut ParserState<'a>| {
                state.net.lam(left, right)
            });
        return choice((era, dup, ctr, var));
    });
}

fn parse_eqn<'a>() -> impl Parser<'a, &'a str, (TermPtr, TermPtr), State<ParserState<'a>>> {
    return parse_term()
        .then_ignore(just('~').padded())
        .then(parse_term());
}

fn parse_eqns<'a>() -> impl Parser<'a, &'a str, (), State<ParserState<'a>>> {
    return parse_eqn()
        .map_with_state(|eqn, _, state| state.net.eqn(eqn.0, eqn.1))
        .separated_by(just('&').padded())
        .collect();
}

fn parse_head<'a>() -> impl Parser<'a, &'a str, (), State<ParserState<'a>>> {
    return parse_term()
        .separated_by(just(',').padded())
        .at_least(1)
        .collect::<Vec<TermPtr>>()
        .delimited_by(just('(').padded(), just(')').padded())
        .map_with_state(|term_ptrs, _, state| {
            for term_ptr in term_ptrs {
                state.net.head(term_ptr)
            }
        });
}

fn parse_def<'a>() -> impl Parser<'a, &'a str, &'a str, State<ParserState<'a>>> {
    return keyword("def")
        .padded()
        .ignore_then(text::ident().padded())
        .then(parse_head())
        .then(just('=').padded().ignore_then(parse_eqns()).or_not())
        .map_with_state(|out, _, state| out.0 .0);
}

// type NetState<'a, I: Input<'a>> = Full<Simple<'a, I>, ParserState<'a>, ()>;

fn parse_book<'a>() -> impl Parser<'a, &'a str, Vec<&'a str>, State<ParserState<'a>>> {
    return parse_def()
        .separated_by(just(';').padded())
        .allow_trailing()
        .collect::<Vec<_>>();
}

#[cfg(test)]
mod tests {
    use crate::strandal::runtime::Runtime;

    use super::*;

    #[test]
    fn test_term() {
        let src = "([* *] a)";
        let mut net = Net::new();
        let mut state = ParserState::new(&mut net);
        let a = parse_term().parse_with_state(src, &mut state);
        assert_eq!(state.vars.contains_key("a"), true);
        println!("{:?}", a);
    }

    #[test]
    fn test_term2() {
        let src = "a";
        let mut net = Net::new();
        let mut state = ParserState::new(&mut net);
        let a = parse_term().parse_with_state(src, &mut state);
        assert_eq!(state.vars.contains_key("a"), true);
        println!("{:?}", a);
    }

    #[test]
    fn test_eqn() {
        let src = "([* *] *) ~ a";
        let mut net = Net::new();
        let mut state = ParserState::new(&mut net);
        let a = parse_eqn().parse_with_state(src, &mut state);
        assert_eq!(state.vars.contains_key("a"), true);
        println!("{:?}", a);
    }

    #[test]
    fn test_eqns() {
        let src = "* ~ * & * ~ R";
        let mut net = Net::new();
        let mut state = ParserState::new(&mut net);
        let a = parse_eqns().parse_with_state(src, &mut state);
        assert_eq!(state.vars.len(), 1);
        println!("{:?}", a);
        println!("{:?}", state.net);
    }

    #[test]
    fn test_head() {
        let src = "(R, *, f)";
        let mut net = Net::new();
        let mut state = ParserState::new(&mut net);
        let a = parse_head().parse_with_state(src, &mut state);
        println!("{:?}", a);
        println!("{:?}", state.net);
    }

    #[test]
    fn test_def() {
        let src = "def a(R, *) = * ~ *";
        let mut net = Net::new();
        let mut state = ParserState::new(&mut net);
        let a = parse_def().parse_with_state(src, &mut state);
        println!("{:?}", a);
        println!("{:?}", state.net);
        println!("{:?}", state.defs);
    }

    #[test]
    fn test_def2() {
        let src = "def a(R, *)";
        let mut net = Net::new();
        let mut state = ParserState::new(&mut net);
        let a = parse_def().parse_with_state(src, &mut state);
        println!("{:?}", a);
        println!("{:?}", state.net);
        println!("{:?}", state.defs);
    }

    #[test]
    fn test_def3() {
        let src = "def a(*)";
        let mut net = Net::new();
        let mut state = ParserState::new(&mut net);
        let a = parse_def().parse_with_state(src, &mut state);
        println!("{:?}", a);
        println!("{:?}", state.net);
        println!("{:?}", state.defs);
    }

    #[test]
    fn test_def4() {
        let src = "def a()"; // ERROR
        let mut net = Net::new();
        let mut state = ParserState::new(&mut net);
        let a = parse_def().parse_with_state(src, &mut state);
        println!("{:?}", a);
        println!("{:?}", state.net);
        println!("{:?}", state.defs);
    }

    #[test]
    fn test_book() {
        let src = "
            def c10([* *])
        "; // ERROR
        let mut net = Net::new();
        let mut state = ParserState::new(&mut net);
        let a = parse_book().parse_with_state(src, &mut state);
        println!("{:?}", a);
        println!("{:?}", state.net);
        println!("{:?}", state.defs);
    }

    #[test]
    fn test_book1() {
        // a@< R | R ~ * >
        let src = "
            def a(R) = * ~ * ;
            def b(*, R) = R ~ * ;
        ";
        let mut net = Net::new();
        let mut state = ParserState::new(&mut net);
        let result = parse_book()
            .parse_with_state(src.trim(), &mut state)
            .into_result();
        match result {
            Ok(_) => {
                println!("Success!! {:?}", state.net);
                println!("{:?}", state.defs);
                let mut runtime = Runtime::new();
                runtime.eval(&mut state.net);
                println!("Executed:  {:?}", state.net);
            }
            Err(errs) => {
                errs.into_iter().for_each(|e| println!("{}", e));
                // println!("{:?}", state.net);
                println!("{:?}", state.defs);
            }
        }
    }
}
