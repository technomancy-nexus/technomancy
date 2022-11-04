use std::collections::HashMap;

use nom::{
    branch::alt,
    character::{
        self,
        complete::{alpha1, char, multispace0, multispace1, none_of, satisfy},
    },
    error::{Error, ErrorKind},
    multi::{many0, many0_count, many1, separated_list0},
    sequence::{delimited, pair, separated_pair, terminated},
    IResult, Parser,
};
use nom_supreme::{error::ErrorTree, final_parser::final_parser, tag::complete::tag, ParserExt};

#[derive(Debug, Clone)]
pub struct ForLoop {
    variable: Variable,
    source: Box<Expression>,
    body: Vec<Expression>,
}

#[derive(Debug, Clone)]
pub struct LetVariable {
    variable: Variable,
    value: Box<Expression>,
}

#[derive(Debug, Clone)]
pub struct Variable(String);

#[derive(Debug, Clone)]
struct DslProgram;

#[derive(Debug, Clone)]
pub enum Expression {
    LetVariable(LetVariable),
    ForLoop(ForLoop),

    Variable(Variable),
    Array(Vec<Expression>),
    MethodCall {
        variable: String,
        method: String,
        arguments: Vec<Expression>,
    },

    Negated(Box<Expression>),
    Multiply(Box<Expression>, Box<Expression>),
    Divide(Box<Expression>, Box<Expression>),
    Add(Box<Expression>, Box<Expression>),
    Substract(Box<Expression>, Box<Expression>),

    Number(i64),
    String(String),
    Void,
    GameObject(GameObject),
}

pub trait MethodFunction {
    fn call(&mut self, args: Vec<Expression>) -> Result<Expression, EvaluationError>;

    fn clone_boxed(&self) -> Box<dyn MethodFunction>;
}

impl<F> MethodFunction for F
where
    F: FnMut(Vec<Expression>) -> Result<Expression, EvaluationError> + Clone + 'static,
{
    fn call(&mut self, args: Vec<Expression>) -> Result<Expression, EvaluationError> {
        (*self)(args)
    }

    fn clone_boxed(&self) -> Box<dyn MethodFunction> {
        Box::new(self.clone())
    }
}

pub struct GameObject {
    pub methods: HashMap<String, Box<dyn MethodFunction>>,
}

impl Clone for GameObject {
    fn clone(&self) -> Self {
        Self {
            methods: self
                .methods
                .iter()
                .map(|(k, v)| (k.clone(), v.clone_boxed()))
                .collect(),
        }
    }
}

impl std::fmt::Debug for GameObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameObject")
            .field("methods", &"...")
            .finish()
    }
}

fn parse_ident(input: &str) -> IResult<&str, &str, ErrorTree<&str>> {
    pair(
        alpha1,
        many0_count(satisfy(|c| {
            ('a'..='z').contains(&c)
                || ('A'..='Z').contains(&c)
                || ('0'..='9').contains(&c)
                || ['_'].contains(&c)
        })),
    )
    .recognize()
    .parse(input)
}

fn parse_number(input: &str) -> IResult<&str, Expression, ErrorTree<&str>> {
    character::complete::i64
        .map(Expression::Number)
        .context("number")
        .parse(input)
}

fn parse_method_call(input: &str) -> IResult<&str, Expression, ErrorTree<&str>> {
    let call = separated_pair(parse_ident, char('.'), parse_ident.cut());

    let args = delimited(
        char('('),
        separated_list0(char(','), parse_expression),
        char(')').cut(),
    );

    call.and(args)
        .map(|((variable, method), arguments)| Expression::MethodCall {
            variable: variable.to_string(),
            method: method.to_string(),
            arguments,
        })
        .parse(input)
}

fn parse_variable(input: &str) -> IResult<&str, Expression, ErrorTree<&str>> {
    parse_ident
        .map(|s| Expression::Variable(Variable(s.to_string())))
        .parse(input)
}

fn parse_arith_atom(input: &str) -> IResult<&str, Expression, ErrorTree<&str>> {
    alt((parse_number, parse_method_call, parse_variable)).parse(input)
}

fn parse_expression(input: &str) -> IResult<&str, Expression, ErrorTree<&str>> {
    let string = many0(none_of("\""))
        .cut()
        .delimited_by(char('"'))
        .map(|s| Expression::String(s.into_iter().collect()))
        .context("string");

    let negate = char('-').precedes(parse_expression).context("negation");

    let array = terminated(
        separated_list0(char(','), parse_expression.context("array value"))
            .cut()
            .preceded_by(char('[')),
        char(']'),
    )
    .map(Expression::Array);

    let void = tag("()").map(|_| Expression::Void).context("void");

    let multiply_divide = parse_arith_atom
        .and(nom::character::complete::one_of("*/").delimited_by(multispace1))
        .and(parse_arith_atom)
        .map_res(|((lhs, kind), rhs)| {
            let ty: fn(Box<Expression>, Box<Expression>) -> Expression = match kind {
                '*' => Expression::Multiply,
                '/' => Expression::Divide,
                _ => return Err::<_, Error<_>>(nom::error_position!(kind, ErrorKind::Tag)),
            };
            Ok((ty)(Box::new(lhs), Box::new(rhs)))
        })
        .context("multiple/divide");

    let add_subtract = parse_arith_atom
        .and(nom::character::complete::one_of("+-").delimited_by(multispace1))
        .and(parse_arith_atom)
        .map_res(|((lhs, kind), rhs)| {
            let ty: fn(Box<Expression>, Box<Expression>) -> Expression = match kind {
                '+' => Expression::Add,
                '-' => Expression::Substract,
                _ => return Err::<_, Error<_>>(nom::error_position!(kind, ErrorKind::Tag)),
            };
            Ok((ty)(Box::new(lhs), Box::new(rhs)))
        })
        .context("add/subtract");

    alt((
        parse_number,
        string,
        negate,
        multiply_divide,
        add_subtract,
        array,
        parse_method_call,
        parse_variable,
        void,
    ))
    .parse(input)
}

fn parse_let_variable(input: &str) -> IResult<&str, Expression, ErrorTree<&str>> {
    multispace1
        .preceded_by(tag("let"))
        .precedes(parse_ident.cut())
        .and(
            char('=')
                .cut()
                .delimited_by(multispace1)
                .precedes(parse_expression.cut()),
        )
        .map(|(variable, value)| {
            Expression::LetVariable(LetVariable {
                variable: Variable(variable.to_string()),
                value: Box::new(value),
            })
        })
        .context("let variable")
        .parse(input)
}

fn parse_statements(input: &str) -> IResult<&str, Vec<Expression>, ErrorTree<&str>> {
    let for_loop = tag("for")
        .precedes(parse_ident.preceded_by(multispace1))
        .and(
            tag("in")
                .delimited_by(multispace1)
                .precedes(parse_expression)
                .cut(),
        )
        .and(delimited(
            char('{').cut().delimited_by(multispace0),
            parse_statements.cut(),
            char('}').cut().delimited_by(multispace0),
        ))
        .map(|((variable, source), body)| {
            Expression::ForLoop(ForLoop {
                variable: Variable(variable.to_string()),
                source: Box::new(source),
                body,
            })
        });

    many1(
        alt((
            terminated(parse_let_variable, char(';').delimited_by(multispace0)),
            terminated(parse_expression, char(';').delimited_by(multispace0)),
            for_loop.delimited_by(multispace0),
        ))
        .delimited_by(multispace0),
    )(input)
}

pub struct EvaluationContext {
    pub values: HashMap<String, Expression>,
}

#[derive(thiserror::Error, Debug)]
pub enum EvaluationError {
    #[error("A value was declared twice: {}", .0)]
    DuplicateValue(String),
    #[error("A variable was not declared before: {}", .0)]
    ValueNotFound(String),
    #[error("A method was not declared before: {}", .0)]
    MethodNotFound(String),
    #[error("An unexpected type was given for the operation")]
    InvalidType,
}

fn eval_expression(
    context: &mut EvaluationContext,
    input: Expression,
) -> Result<Expression, EvaluationError> {
    match input {
        Expression::LetVariable(LetVariable { variable, value }) => {
            let value = eval_expression(context, *value)?;
            if let Some(_) = context.values.insert(variable.0.to_string(), value) {
                return Err(EvaluationError::DuplicateValue(variable.0.to_string()));
            };

            Ok(Expression::Void)
        }
        Expression::Variable(Variable(var)) => {
            Ok(Expression::clone(context.values.get(&var).ok_or_else(
                || EvaluationError::ValueNotFound(var.to_string()),
            )?))
        }
        Expression::ForLoop(ForLoop {
            variable,
            source,
            body,
        }) => {
            let source = eval_expression(context, *source)?;
            match source {
                Expression::Array(values) => {
                    if context.values.remove(&variable.0).is_some() {
                        return Err(EvaluationError::DuplicateValue(variable.0.to_string()));
                    }

                    for val in values {
                        let val = eval_expression(context, val)?;
                        context.values.insert(variable.0.clone(), val);

                        let _: Vec<_> = body
                            .clone()
                            .into_iter()
                            .map(|body| eval_expression(context, body))
                            .collect::<Result<_, _>>()?;
                    }

                    context.values.remove(&variable.0);

                    Ok(Expression::Void)
                }
                _ => return Err(EvaluationError::InvalidType),
            }
        }
        Expression::Negated(val) => {
            let val = eval_expression(context, *val)?;

            match val {
                Expression::Number(num) => Ok(Expression::Number(-num)),
                _ => return Err(EvaluationError::InvalidType),
            }
        }
        Expression::Multiply(lhs, rhs) => {
            let lhs = eval_expression(context, *lhs)?;
            let rhs = eval_expression(context, *rhs)?;

            match (lhs, rhs) {
                (Expression::Number(lhs), Expression::Number(rhs)) => {
                    Ok(Expression::Number(lhs.saturating_mul(rhs)))
                }
                _ => return Err(EvaluationError::InvalidType),
            }
        }
        Expression::Divide(lhs, rhs) => {
            let lhs = eval_expression(context, *lhs)?;
            let rhs = eval_expression(context, *rhs)?;

            match (lhs, rhs) {
                (Expression::Number(lhs), Expression::Number(rhs)) => {
                    Ok(Expression::Number(lhs.saturating_div(rhs)))
                }
                _ => return Err(EvaluationError::InvalidType),
            }
        }
        Expression::Add(lhs, rhs) => {
            let lhs = eval_expression(context, *lhs)?;
            let rhs = eval_expression(context, *rhs)?;

            match (lhs, rhs) {
                (Expression::Number(lhs), Expression::Number(rhs)) => {
                    Ok(Expression::Number(lhs.saturating_add(rhs)))
                }
                _ => return Err(EvaluationError::InvalidType),
            }
        }
        Expression::Substract(lhs, rhs) => {
            let lhs = eval_expression(context, *lhs)?;
            let rhs = eval_expression(context, *rhs)?;

            match (lhs, rhs) {
                (Expression::Number(lhs), Expression::Number(rhs)) => {
                    Ok(Expression::Number(lhs.saturating_sub(rhs)))
                }
                _ => return Err(EvaluationError::InvalidType),
            }
        }
        Expression::MethodCall {
            variable,
            method,
            arguments,
        } => {
            let arguments = arguments
                .into_iter()
                .map(|input| eval_expression(context, input))
                .collect::<Result<_, _>>()?;

            let var = context
                .values
                .get_mut(&variable)
                .ok_or_else(|| EvaluationError::ValueNotFound(variable.to_string()))?;
            let meth = match var {
                Expression::GameObject(obj) => obj
                    .methods
                    .get_mut(&method)
                    .ok_or_else(|| EvaluationError::MethodNotFound(method.to_string()))?,
                _ => return Err(EvaluationError::InvalidType),
            };

            meth.call(arguments)
        }
        expr @ Expression::Number(_)
        | expr @ Expression::GameObject(_)
        | expr @ Expression::Void
        | expr @ Expression::Array(_)
        | expr @ Expression::String(_) => Ok(expr),
    }
}

#[derive(Debug, Clone)]
pub struct GameDsl(Vec<Expression>);

impl GameDsl {
    pub fn parse_from(input: &str) -> Result<Self, ErrorTree<&str>> {
        let stmts = final_parser(parse_statements)(input)?;

        Ok(GameDsl(stmts))
    }

    pub fn evaluate(self, context: &mut EvaluationContext) -> Result<(), EvaluationError> {
        for expr in self.0 {
            eval_expression(context, expr)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use assert_matches::assert_matches;
    use nom::{
        character::complete::multispace0, multi::many1, sequence::terminated, Finish, Parser,
    };
    use nom_supreme::ParserExt;

    use crate::game_dsl::parse_statements;

    use super::{
        eval_expression, parse_let_variable, EvaluationContext, Expression, GameDsl, GameObject,
        MethodFunction,
    };

    #[test]
    fn check_let_parsing() {
        parse_let_variable("").unwrap_err();
        assert_matches!(parse_let_variable("let a = 100").unwrap(), ("", _));
        assert_matches!(parse_let_variable("let b = g + j").unwrap(), ("", _));

        let mut multi_line = many1(
            terminated(
                parse_let_variable,
                nom::character::complete::char(';').delimited_by(multispace0),
            )
            .delimited_by(multispace0),
        );

        let res = multi_line
            .parse("let a = 100; let b = a + 100; let c = a * b;")
            .unwrap();
        assert_matches!(res, ("", _));
    }

    #[test]
    fn check_simple_parse() {
        let input = r#"
            let foo = [12, 45, ["hi"], 14];
            for a in foo {
                game.print(a);
            }
        "#;
        let parsed = parse_statements(input);

        println!("In: {} \n\nOut: {:#?}", input, parsed);
    }

    #[test]
    fn check_evaluation() {
        let input = r#"
            let b = 1000;
            let a = b + 234;
            game.print(a);
        "#;

        let val = Rc::new(RefCell::new(Expression::Void));

        let final_val = val.clone();
        let print_method: Box<dyn MethodFunction> = Box::new(move |args: Vec<Expression>| {
            *val.borrow_mut() = args[0].clone();
            Ok(Expression::Void)
        });
        let game = GameObject {
            methods: [("print".to_string(), print_method)].into_iter().collect(),
        };

        let (rest, parsed) = parse_statements(input).finish().unwrap();

        assert_eq!(rest, "");

        let mut context = EvaluationContext {
            values: [("game".to_string(), Expression::GameObject(game))]
                .into_iter()
                .collect(),
        };
        for val in parsed {
            println!("Evaluation: {:?}", val);
            eval_expression(&mut context, val).unwrap();
        }

        println!("{:?}", final_val.borrow());

        assert_matches!(*final_val.borrow(), Expression::Number(1234));
    }

    #[test]
    fn parse_complex() {
        let input = r#"
            for player in game.all_players() {
                let deck = player.get_zone("deck");
                let hand_cards = deck.take_cards_from_top(7);
                let hand = players.get_zone("hand");
                hand.add_cards_to_start(hand_cards);
            }
        "#;

        let res = parse_statements(input);

        println!("{:#?}", res);

        assert_matches!(res, Ok(("", _)));
    }
}
