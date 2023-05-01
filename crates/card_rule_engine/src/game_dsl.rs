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

use crate::game_state::GameState;

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

impl Expression {
    pub fn get_kind(&self, context: &EvaluationContext) -> Result<ExpressionKind, EvaluationError> {
        match self {
            Expression::LetVariable(LetVariable { variable: _, value }) => value.get_kind(context),
            Expression::ForLoop(_) => Ok(ExpressionKind::Void),
            Expression::Variable(variable) => context
                .values
                .get(&variable.0)
                .ok_or_else(|| EvaluationError::ValueNotFound(variable.0.to_string()))
                .and_then(|val| val.get_kind(context)),
            Expression::Array(arr) => Ok(arr
                .first()
                .map(|val| val.get_kind(context))
                .transpose()?
                .unwrap_or(ExpressionKind::Void)),
            Expression::MethodCall {
                variable,
                method,
                arguments,
            } => {
                let obj = if let Some(obj) = context.values.get(variable) {
                    obj
                } else {
                    return Err(EvaluationError::ValueNotFound(variable.to_string()));
                };

                let method = if let Expression::GameObject(GameObject { methods, .. }) = obj {
                    if let Some(method) = methods.get(method) {
                        method
                    } else {
                        return Err(EvaluationError::MethodNotFound(method.to_string()));
                    }
                } else {
                    return Err(EvaluationError::InvalidType {
                        expected: ExpressionKind::GameObject(String::new()),
                        found: ExpressionKind::Void,
                    });
                };

                Ok(method.kind_signature.clone())
            }
            Expression::Negated(_) => todo!(),
            Expression::Multiply(_, _) => todo!(),
            Expression::Divide(_, _) => todo!(),
            Expression::Add(_, _) => todo!(),
            Expression::Substract(_, _) => todo!(),
            Expression::Number(_) => todo!(),
            Expression::String(_) => todo!(),
            Expression::Void => todo!(),
            Expression::GameObject(GameObject { kind: name, .. }) => {
                Ok(ExpressionKind::GameObject(name.to_string()))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionKind {
    Number,
    String,
    Void,
    GameObject(String),

    MethodCall {
        arguments: Vec<ExpressionKind>,
        return_kind: Box<ExpressionKind>,
    },
    Array(Box<ExpressionKind>),
}

pub trait SimpleMethod {
    fn call(&self, state: &GameState, args: Vec<Expression>)
        -> Result<Expression, EvaluationError>;
}

pub trait MethodFunction: SimpleMethod {
    fn clone_boxed(&self) -> Box<dyn MethodFunction>;
}

impl<F: Clone + SimpleMethod + 'static> MethodFunction for F {
    fn clone_boxed(&self) -> Box<dyn MethodFunction> {
        Box::new(self.clone())
    }
}

pub struct Method {
    implementation: Box<dyn MethodFunction>,
    kind_signature: ExpressionKind,
}

impl Clone for Method {
    fn clone(&self) -> Self {
        Self {
            implementation: self.implementation.clone_boxed(),
            kind_signature: self.kind_signature.clone(),
        }
    }
}

impl Method {
    pub const fn new(function: Box<dyn MethodFunction>, kind_signature: ExpressionKind) -> Method {
        assert!(matches!(kind_signature, ExpressionKind::MethodCall { .. }));

        Method {
            implementation: function,
            kind_signature,
        }
    }
}

#[derive(Clone)]
pub struct GameObject {
    pub kind: String,
    pub methods: HashMap<String, Method>,
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
    InvalidType {
        expected: ExpressionKind,
        found: ExpressionKind,
    },
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
                expr => {
                    return Err(EvaluationError::InvalidType {
                        expected: ExpressionKind::Array(Box::new(ExpressionKind::Void)),
                        found: expr.get_kind(context)?,
                    })
                }
            }
        }
        Expression::Negated(val) => {
            let val = eval_expression(context, *val)?;

            match val {
                Expression::Number(num) => Ok(Expression::Number(-num)),
                expr => {
                    return Err(EvaluationError::InvalidType {
                        expected: ExpressionKind::Number,
                        found: expr.get_kind(context)?,
                    })
                }
            }
        }
        Expression::Multiply(lhs, rhs) => {
            let lhs = eval_expression(context, *lhs)?;
            let rhs = eval_expression(context, *rhs)?;

            match (lhs, rhs) {
                (Expression::Number(lhs), Expression::Number(rhs)) => {
                    Ok(Expression::Number(lhs.saturating_mul(rhs)))
                }
                (Expression::Number(_), other) | (other, Expression::Number(_)) => {
                    return Err(EvaluationError::InvalidType {
                        expected: ExpressionKind::Number,
                        found: other.get_kind(context)?,
                    })
                }
                (other, _) => {
                    return Err(EvaluationError::InvalidType {
                        expected: ExpressionKind::Number,
                        found: other.get_kind(context)?,
                    })
                }
            }
        }
        Expression::Divide(lhs, rhs) => {
            let lhs = eval_expression(context, *lhs)?;
            let rhs = eval_expression(context, *rhs)?;

            match (lhs, rhs) {
                (Expression::Number(lhs), Expression::Number(rhs)) => {
                    Ok(Expression::Number(lhs.saturating_div(rhs)))
                }
                (Expression::Number(_), other) | (other, Expression::Number(_)) => {
                    return Err(EvaluationError::InvalidType {
                        expected: ExpressionKind::Number,
                        found: other.get_kind(context)?,
                    })
                }
                (other, _) => {
                    return Err(EvaluationError::InvalidType {
                        expected: ExpressionKind::Number,
                        found: other.get_kind(context)?,
                    })
                }
            }
        }
        Expression::Add(lhs, rhs) => {
            let lhs = eval_expression(context, *lhs)?;
            let rhs = eval_expression(context, *rhs)?;

            match (lhs, rhs) {
                (Expression::Number(lhs), Expression::Number(rhs)) => {
                    Ok(Expression::Number(lhs.saturating_add(rhs)))
                }
                (Expression::Number(_), other) | (other, Expression::Number(_)) => {
                    return Err(EvaluationError::InvalidType {
                        expected: ExpressionKind::Number,
                        found: other.get_kind(context)?,
                    })
                }
                (other, _) => {
                    return Err(EvaluationError::InvalidType {
                        expected: ExpressionKind::Number,
                        found: other.get_kind(context)?,
                    })
                }
            }
        }
        Expression::Substract(lhs, rhs) => {
            let lhs = eval_expression(context, *lhs)?;
            let rhs = eval_expression(context, *rhs)?;

            match (lhs, rhs) {
                (Expression::Number(lhs), Expression::Number(rhs)) => {
                    Ok(Expression::Number(lhs.saturating_sub(rhs)))
                }
                (Expression::Number(_), other) | (other, Expression::Number(_)) => {
                    return Err(EvaluationError::InvalidType {
                        expected: ExpressionKind::Number,
                        found: other.get_kind(context)?,
                    })
                }
                (other, _) => {
                    return Err(EvaluationError::InvalidType {
                        expected: ExpressionKind::Number,
                        found: other.get_kind(context)?,
                    })
                }
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
                .get(&variable)
                .ok_or_else(|| EvaluationError::ValueNotFound(variable.to_string()))?;
            dbg!(var.get_kind(context)?);
            dbg!(&method);
            let method = match var {
                Expression::GameObject(obj) => obj
                    .methods
                    .get(&method)
                    .ok_or_else(|| EvaluationError::MethodNotFound(method.to_string()))?,
                expr => {
                    return Err(EvaluationError::InvalidType {
                        expected: ExpressionKind::GameObject(String::from("<any gameobject>")),
                        found: expr.get_kind(context)?,
                    })
                }
            };

            method.implementation.call(arguments)
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

    use crate::game_dsl::{parse_statements, ExpressionKind, Method, SimpleMethod};

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
        let print_method: Box<dyn MethodFunction> = Box::new({
            #[derive(Debug, Clone)]
            struct SimpleBorrow(Rc<RefCell<Expression>>);

            impl SimpleMethod for SimpleBorrow {
                fn call(
                    &self,
                    mut args: Vec<Expression>,
                ) -> Result<Expression, crate::game_dsl::EvaluationError> {
                    *self.0.borrow_mut() = args.pop().unwrap();

                    Ok(Expression::Void)
                }
            }

            SimpleBorrow(val)
        });
        let game = GameObject {
            kind: "game".to_string(),
            methods: [(
                "print".to_string(),
                Method::new(
                    print_method,
                    ExpressionKind::MethodCall {
                        arguments: vec![ExpressionKind::Number],
                        return_kind: Box::new(ExpressionKind::Void),
                    },
                ),
            )]
            .into_iter()
            .collect(),
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
