use crate::ast::expression;
use crate::errors::{ErrorBuilder, Error, ErrorType};
use crate::lex::tokens::{Token, TokenType};
use std::iter::Peekable;
use std::convert::TryInto;
use crate::ast::expression::Expression;

pub struct Parser<'a, T: Iterator<Item = &'a Token>> {
    tokens: Peekable<T>,
    err_build: &'a ErrorBuilder,
}

impl<'a, T: Iterator<Item = &'a Token>> Parser<'a, T> {
    pub fn new(tokens: T, err_build: &'a ErrorBuilder) -> Parser<'a, T> {
        Parser {
            tokens: tokens.peekable(),
            err_build,
        }
    }

    pub fn parse(&mut self) -> Result<Vec<expression::Expression>, Error> {
        let exprs = self.read_multiple_exprs()?;
        Ok(exprs)
    }

    fn read_multiple_exprs(&mut self) -> Result<Vec<expression::Expression>, Error> {
        let mut exprs = vec![];
        while let Some(_) = self.tokens.peek() {
            let expr = self.statement()?;
            if let expression::Expression::Statement(ex) = &expr {
                exprs.push(expr);
                continue;
            }
            exprs.push(expr);
            break;
        }
        Ok(exprs)
    }

    fn expression(&mut self) -> Result<expression::Expression, Error> {
        self.assign()
    }

    fn statement(&mut self) -> Result<expression::Expression, Error> {
        let mut expr = self.expression()?;
        if let Some(Token {token_type: TokenType::SemiColon, ..}) = self.tokens.peek() {
            self.tokens.next();
            Ok(expression::Expression::Statement(Box::new(expr)))
        } else {
            Err(self.err_build.create(0, 0, ErrorType::MissingSemiColon))
        }
    }

    fn assign(&mut self) -> Result<expression::Expression, Error> {
        let mut l = self.print()?;
        if let Some(Token {token_type: TokenType::Equal, ..}) = self.tokens.peek() {
            self.tokens.next();
            let right = self.assign()?;
            if let Expression::Variable(name) = l {
                return Ok(Expression::Assign(name, Box::from(right)));
            } else {
                return Err(self.err_build.create(0, 0, ErrorType::InvalidAssignment))
            }
        }
        Ok(l)
    }

    // TODO remove print statement in favour of a function for various reasons
    fn print(&mut self) -> Result<expression::Expression, Error> {
        if let Some(Token {token_type: TokenType::Identifier(ident), ..}) = self.tokens.peek() {
            if ident == "print" {
                self.tokens.next();
                return Ok(expression::Expression::Print(Box::new(self.equality()?)));
            }
        }
        self.equality()
    }

    fn binary_helper(&mut self, matcher: impl Fn(&TokenType)->bool, left: expression::Expression, right: fn(&mut Parser<'a, T>)->Result<expression::Expression, Error>) -> Result<expression::Expression, Error> {
        let mut expr = left;
        loop {
            match self.tokens.peek() {
                Some(Token { token_type: x, .. })
                if matcher(x) =>
                    {
                        let token = self.tokens.next().unwrap().try_into().unwrap();
                        let expr_right = right(self);
                        expr = expression::Expression::Binary {
                            kind: token,
                            operands: (Box::new(expr), Box::new(expr_right?))
                        };
                    },
                _ => break
            }
        }
        Ok(expr)
    }

    fn equality(&mut self) -> Result<expression::Expression, Error> {
        let expr = self.comparison()?;
        self.binary_helper(|x|*x == TokenType::BangEqual || *x == TokenType::EqualEqual, expr, Parser::comparison)
    }

    fn comparison(&mut self) -> Result<expression::Expression, Error> {
        let expr = self.addition()?;
        self.binary_helper(|x| *x == TokenType::Greater || *x == TokenType::GreaterEqual || *x == TokenType::Lesser || *x == TokenType::LesserEqual, expr, Parser::addition)
    }

    fn addition(&mut self) -> Result<expression::Expression, Error> {
        let expr = self.multiplication()?;
        self.binary_helper(|x| *x == TokenType::Minus || *x == TokenType::Plus, expr, Parser::multiplication)
    }

    fn multiplication(&mut self) -> Result<expression::Expression, Error> {
        let expr = self.unary()?;
        self.binary_helper(|x| *x == TokenType::Slash || *x == TokenType::Star, expr, Parser::unary)
    }

    fn unary(&mut self) -> Result<expression::Expression, Error> {
        match self.tokens.peek() {
            Some(Token{token_type: x, ..}) if *x == TokenType::Bang || *x == TokenType::Minus => {
                Ok(expression::Expression::Unary {
                    kind: self.tokens.next().unwrap().try_into().unwrap(),
                    expr: Box::new(self.unary()?)
                })
            },
            _ => self.primary()
        }
    }

    fn primary(&mut self) -> Result<expression::Expression, Error> {
        if let Some(token) = self.tokens.next() {
            match &token.token_type {
                TokenType::True => Ok(expression::Expression::Literal(expression::Literal::True)),
                TokenType::False => Ok(expression::Expression::Literal(expression::Literal::False)),
                TokenType::Integer(i)=> Ok(expression::Expression::Literal(expression::Literal::Integer(*i))),
                TokenType::Float(f) => Ok(expression::Expression::Literal(expression::Literal::Float(*f))),
                TokenType::String(s) => Ok(expression::Expression::Literal(expression::Literal::String(s.to_string()))),
                TokenType::LeftParen => {
                    let expr = self.expression()?;
                    if let Some(Token{token_type: TokenType::RightParen, ..}) = self.tokens.next() {
                        Ok(expression::Expression::Grouping(Box::new(expr)))
                    } else {
                        Err(self.err_build.create(token.location, 1, ErrorType::UnclosedParen))
                    }
                },
                TokenType::Identifier(s) => Ok(Expression::Variable(s.to_string())),
                x => Err(self.err_build.create(token.location, 1, ErrorType::UnexpectedToken(x.clone())))
            }
        } else {
            Err(self.err_build.create(0, 0, ErrorType::UnexpectedEOF))
        }

    }
}
