use nom::{
    branch::alt,
    character::{
        self,
        complete::{alpha1, alphanumeric0, anychar, char, multispace0, none_of},
    },
    multi::{many0, many1, separated_list0, separated_list1},
    sequence::{delimited, pair, separated_pair, terminated},
    IResult, Parser,
};
use nom_supreme::{error::ErrorTree, tag::complete::tag, ParserExt};

#[derive(Debug, Clone)]
struct ForLoop {
    variable: Variable,
    source: Box<Expression>,
    body: Vec<Expression>,
}

#[derive(Debug, Clone)]
struct LetVariable {
    variable: Variable,
    value: Box<Expression>,
}

#[derive(Debug, Clone)]
struct Variable(String);

#[derive(Debug, Clone)]
struct DslProgram;

#[derive(Debug, Clone)]
enum Expression {
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
}

fn parse_ident(input: &str) -> IResult<&str, &str, ErrorTree<&str>> {
    pair(alpha1, alphanumeric0).recognize().parse(input)
}

fn parse_expression(input: &str) -> IResult<&str, Expression, ErrorTree<&str>> {
    let number = character::complete::i64
        .map(Expression::Number)
        .context("number");

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

    let method_call = {
        let call = separated_pair(parse_ident, char('.'), parse_ident);

        let args = delimited(
            char('('),
            separated_list0(char(','), parse_expression),
            char(')'),
        );

        call.and(args)
    }
    .map(|((variable, method), arguments)| Expression::MethodCall {
        variable: variable.to_string(),
        method: method.to_string(),
        arguments,
    });

    let variable = parse_ident.map(|s| Expression::Variable(Variable(s.to_string())));

    alt((number, string, negate, array, method_call, variable))
        .delimited_by(multispace0)
        .context("expression")
        .parse(input)
}

fn parse_statements(input: &str) -> IResult<&str, Vec<Expression>, ErrorTree<&str>> {
    let let_variable = tag("let")
        .delimited_by(multispace0)
        .precedes(parse_ident)
        .and(
            char('=')
                .delimited_by(multispace0)
                .precedes(parse_expression),
        )
        .map(|(variable, value)| {
            Expression::LetVariable(LetVariable {
                variable: Variable(variable.to_string()),
                value: Box::new(value),
            })
        })
        .context("let variable");

    let for_loop = tag("for")
        .delimited_by(multispace0)
        .precedes(parse_ident.cut())
        .and(
            tag("in")
                .delimited_by(multispace0)
                .precedes(parse_expression)
                .cut(),
        )
        .delimited_by(multispace0)
        .and(delimited(char('{'), parse_statements.cut(), char('}')))
        .map(|((variable, source), body)| {
            Expression::ForLoop(ForLoop {
                variable: Variable(variable.to_string()),
                source: Box::new(source),
                body,
            })
        });

    many0(
        alt((
            terminated(let_variable, char(';')),
            terminated(parse_expression, char(';')),
            for_loop,
        ))
        .delimited_by(multispace0),
    )(input)
}

struct EvaluationResult {
    final_expression: Expression,
}

fn eval_expression(input: Expression) -> EvaluationResult {
    match input {
        Expression::LetVariable(_) => todo!(),
        Expression::Variable(_) => todo!(),
        Expression::ForLoop(_) => todo!(),
        Expression::Negated(_) => todo!(),
        Expression::Multiply(_, _) => todo!(),
        Expression::Divide(_, _) => todo!(),
        Expression::Add(_, _) => todo!(),
        Expression::Substract(_, _) => todo!(),
        Expression::Number(_) => todo!(),
        Expression::Array(_) => todo!(),
        Expression::String(_) => todo!(),
        Expression::MethodCall {
            variable: _,
            method: _,
            arguments: _,
        } => todo!(),
    }
}

#[derive(Debug, Clone)]
pub struct GameDsl(Expression);

#[cfg(test)]
mod tests {
    use crate::game_dsl::parse_statements;

    use super::parse_expression;

    #[test]
    fn check_simple_parse() {
        let input = "for a in [2, 3 , 4] { a + 1 }";
        let input = r#"
            let foo = [12, 45, ["hi"], 14];
            for a in foo {
                game.print(a);
            }
        "#;
        let parsed = parse_statements(input);

        println!("In: {} \n\nOut: {:#?}", input, parsed);
        panic!();
    }
}
