use std::num::TryFromIntError;

use nom::{
    self,
    branch::{alt, permutation},
    bytes::complete::{is_not, tag, take_until},
    character::complete::{anychar, char, none_of, one_of, satisfy},
    combinator::{map, map_opt, map_res, recognize, value},
    multi::{fold_many0, fold_many1, many0, many0_count, many1_count, separated_list0},
    number::complete::hex_u32,
    sequence::{delimited, pair, preceded, separated_pair, tuple},
    AsChar, IResult,
};

use crate::number::Number;

#[derive(Debug, Clone)]
pub enum Token {
    Identifier,
    Boolean(bool),
    Number(Number),
    Character(char),
    String(String),
    OpenParen,
    CloseParen,
    OpenVec,
    OpenByteVec,
    Quote,
    BackQuote,
    Comma,
    CommaAt,
    Period,
}

pub fn lex(i: &str) -> IResult<&str, Vec<Token>> {
    many0(delimited(intertoken_space, token, intertoken_space))(i)
}

pub fn token(i: &str) -> IResult<&str, Token> {
    use Token::*;
    alt((
        map(boolean, Boolean),
        map(number, Number),
        value(Identifier, identifier),
        map(character, Character),
        map(string, String),
        value(OpenParen, tag("(")),
        value(CloseParen, tag(")")),
        value(OpenVec, tag("#(")),
        value(OpenByteVec, tag("#u8(")),
        value(Quote, tag("'")),
        value(BackQuote, tag("`")),
        value(CommaAt, tag(",@")),
        value(Comma, tag(",")),
        value(Period, tag(",")),
    ))(i)
}

fn delimiter(i: &str) -> IResult<&str, &str> {
    alt((
        whitespace,
        tag("|"),
        tag("("),
        tag(")"),
        tag("\""),
        tag(";"),
    ))(i)
}

fn intraline_whitespace(i: &str) -> IResult<&str, char> {
    one_of(&[' ', '\t'] as &[char])(i)
}

fn whitespace(i: &str) -> IResult<&str, &str> {
    alt((recognize(intraline_whitespace), line_ending))(i)
}

fn line_ending(i: &str) -> IResult<&str, &str> {
    alt((tag("\n"), tag("\r\n"), tag("\r")))(i)
}

fn comment(i: &str) -> IResult<&str, &str> {
    alt((
        preceded(char(';'), is_not("\n\r")),
        nested_comment,
        preceded(pair(tag("#;"), intertoken_space), datum),
    ))(i)
}

fn nested_comment(i: &str) -> IResult<&str, &str> {
    delimited(
        tag("#|"),
        recognize(pair(comment_text, many0_count(comment_cont))),
        tag("|#"),
    )(i)
}

fn comment_text(i: &str) -> IResult<&str, &str> {
    // TODO: support nesting
    take_until("|#")(i)
}

fn comment_cont(i: &str) -> IResult<&str, &str> {
    recognize(pair(nested_comment, comment_text))(i)
}

fn directive(i: &str) -> IResult<&str, &str> {
    alt((tag("#!fold-case"), tag("#!no-fold-case")))(i)
}

fn atmosphere(i: &str) -> IResult<&str, &str> {
    alt((whitespace, comment, directive))(i)
}

fn intertoken_space(i: &str) -> IResult<&str, &str> {
    recognize(many0_count(atmosphere))(i)
}

fn identifier(i: &str) -> IResult<&str, &str> {
    alt((
        recognize(pair(initial, many0_count(subsequent))),
        delimited(tag("|"), recognize(many0_count(symbol_element)), tag("|")),
        peculiar_identifier,
    ))(i)
}

fn initial(i: &str) -> IResult<&str, char> {
    alt((letter, special_initial))(i)
}

fn letter(i: &str) -> IResult<&str, char> {
    satisfy(AsChar::is_alpha)(i)
}

fn special_initial(i: &str) -> IResult<&str, char> {
    one_of(&[
        '!', '$', '%', '&', '*', '/', ':', '<', '=', '>', '?', '^', '_', '~',
    ] as &[char])(i)
}

fn subsequent(i: &str) -> IResult<&str, char> {
    alt((
        initial,
        map(digit::<10>, AsChar::as_char),
        special_subsequent,
    ))(i)
}

fn explicit_sign(i: &str) -> IResult<&str, char> {
    one_of(&['+', '-'] as &[char])(i)
}

fn special_subsequent(i: &str) -> IResult<&str, char> {
    alt((explicit_sign, char('.'), char('@')))(i)
}

fn inline_hex_escape(i: &str) -> IResult<&str, char> {
    preceded(tag(r"\x"), hex_scalar_value)(i)
}

fn hex_scalar_value(i: &str) -> IResult<&str, char> {
    map_opt(
        fold_many1(digit::<16>, || 0u32, |acc, dig| acc * 16 + dig as u32),
        char::from_u32,
    )(i)
}

fn mnemonic_escape(i: &str) -> IResult<&str, char> {
    alt((
        value('\x07', tag(r"\a")),
        value('\x08', tag(r"\b")),
        value('\t', tag(r"\t")),
        value('\n', tag(r"\n")),
        value('\r', tag(r"\r")),
    ))(i)
}

fn peculiar_identifier(i: &str) -> IResult<&str, &str> {
    alt((
        recognize(explicit_sign),
        recognize(tuple((
            explicit_sign,
            tag("."),
            dot_subsequent,
            many0(subsequent),
        ))),
        recognize(tuple((tag("."), dot_subsequent, many0(subsequent)))),
    ))(i)
}

fn dot_subsequent(i: &str) -> IResult<&str, char> {
    alt((sign_subsequent, char('.')))(i)
}

fn sign_subsequent(i: &str) -> IResult<&str, char> {
    alt((initial, explicit_sign, char('@')))(i)
}

fn symbol_element(i: &str) -> IResult<&str, char> {
    alt((
        none_of(&['|', '\\'] as &[char]),
        inline_hex_escape,
        mnemonic_escape,
        value('|', tag(r"\|")),
    ))(i)
}

fn boolean(i: &str) -> IResult<&str, bool> {
    alt((
        value(true, tag("#true")),
        value(false, tag("#false")),
        value(true, tag("#t")),
        value(false, tag("#f")),
    ))(i)
}

fn character(i: &str) -> IResult<&str, char> {
    alt((
        preceded(tag(r"#\"), anychar),
        preceded(tag(r"#\"), character_name),
        preceded(tag(r"#\x"), hex_scalar_value),
    ))(i)
}

fn character_name(i: &str) -> IResult<&str, char> {
    alt((
        value('\x07', tag("alarm")),
        value('\x08', tag("backspace")),
        value('\x1B', tag("delete")),
        value('\n', tag("newline")),
        value('\0', tag("null")),
        value('\r', tag("return")),
        value(' ', tag("space")),
        value('\t', tag("tab")),
    ))(i)
}

fn string(i: &str) -> IResult<&str, String> {
    delimited(
        tag("\""),
        fold_many0(
            string_element,
            || String::with_capacity(16),
            |mut acc, c| {
                acc.extend(c);
                acc
            },
        ),
        tag("\""),
    )(i)
}

fn string_element(i: &str) -> IResult<&str, Option<char>> {
    alt((
        map(none_of(&['"', '\\'] as &[char]), Some),
        map(mnemonic_escape, Some),
        value(Some('"'), tag(r#"\""#)),
        value(Some('\\'), tag(r#"\\"#)),
        value(
            None,
            recognize(tuple((
                tag(r"\"),
                many0_count(intraline_whitespace),
                line_ending,
                many0_count(intraline_whitespace),
            ))),
        ),
        map(inline_hex_escape, Some),
    ))(i)
}

fn bytevector(i: &str) -> IResult<&str, &str> {
    delimited(tag("#u8("), recognize(many0_count(byte)), tag(")"))(i)
}

fn byte(i: &str) -> IResult<&str, u8> {
    map_res(number, |x| match x {
        Number::Integer(i) => u8::try_from(i).map_err(|e| format!("can't cast to u8: {e}")),
        _ => Err(format!(
            "{x:?} is not an int and can't go in a byte vector!"
        )),
    })(i)
}

fn number(i: &str) -> IResult<&str, Number> {
    // this is quite a silly way to parse it,
    // because we end up recognizing the radix only to discard it later,
    // but oh well
    alt((num::<2>, num::<8>, num::<10>, num::<16>))(i)
}

fn num<const R: u8>(i: &str) -> IResult<&str, Number> {
    map(pair(prefix::<R>, complex::<R>), |(exactness, num)| {
        use Number::*;
        match (exactness, num) {
            (Inexact, Integer(i)) => Real(i as f64),
            (Inexact, Real(x)) => Real(x),
            (Inexact, Rational{num, den}) => Real(num as f64 / den as f64),
            (Exact, Integer(x)) => Integer(x),
            (Exact, Real(x)) => {
                if x as i64 as f64 == x {
                    Integer(x as i64)
                } else {
                    todo!("idk")
                }
            }
            (Exact, Rational{num, den}) => Rational{num, den},
            (Unspecified, x) => x,
        }
    })(i)
}

fn complex<const R: u8>(i: &str) -> IResult<&str, Number> {
    // TODO: support complex numbers
    real::<R>(i)
}

fn real<const R: u8>(i: &str) -> IResult<&str, Number> {
    alt((
        map(pair(sign, ureal::<R>), |t| match t {
            ("-", x) => -x,
            (_, x) => x,
        }),
        infnan,
    ))(i)
}

fn ureal<const R: u8>(i: &str) -> IResult<&str, Number> {
    alt((
        map_res(
            separated_pair(uinteger::<R>, tag("/"), uinteger::<R>),
            |(num, den)| u32::try_from(den).map(|den| Number::Rational { num, den })
        ),
        decimal::<R>,
        map(uinteger::<R>, Number::Integer),
    ))(i)
}

fn decimal<const R: u8>(i: &str) -> IResult<&str, Number> {
    match R {
        10 => alt((
            map_res (
                pair(
                    recognize(delimited(
                        many1_count(digit::<10>),
                        tag("."),
                        many0_count(digit::<10>),
                    )),
                    suffix,
                ),
                |(d,s)| {
                    d.parse::<f64>().map(|f| Number::Real(f * s as f64))
                }
            ),
            map_res(
                recognize(preceded(tag("."), pair(many1_count(digit::<10>), suffix))),
                |d| d.parse::<f64>().map(Number::Real)
            ),
            map_opt (
                pair(uinteger::<10>, suffix),
                |(i, s)| {
                    Some(match i.checked_mul(s) {
                        Some(n) => Number::Integer(n),
                        None => Number::Real(i as f64 * s as f64),
                    })
                }
            ),
        ))(i),
        _ => nom::combinator::fail(i),
    }
}

fn uinteger<const R: u8>(i: &str) -> IResult<&str, i64> {
    map_res (
        fold_many1(
            digit::<R>,
            || Some(0i64),
            |acc, dig| acc?.checked_mul(R as i64)?.checked_add(dig as i64)
        ),
        |o| o.ok_or_else(|| format!("integer literal {i} too large for i64!"))
    )(i)
}

fn prefix<const R: u8>(i: &str) -> IResult<&str, Exactness> {
    map(permutation((radix::<R>, exactness)), |(_, exactness)| {
        exactness
    })(i)
}

fn infnan(i: &str) -> IResult<&str, Number> {
    map(
        alt((
            value(f64::INFINITY, tag("+inf.0")),
            value(f64::NEG_INFINITY, tag("-inf.0")),
            value(f64::NAN, tag("+nan.0")),
            value(f64::NAN, tag("-nan.0")),
        )),
        Number::Real,
    )(i)
}

fn suffix(i: &str) -> IResult<&str, i64> {
    alt((
        preceded(
            tag("e"),
            map_opt (
                recognize(pair(sign, many1_count(digit::<10>))),
                |s| 10i64.checked_pow(s.parse::<u32>().ok()?)
            )
        ),
        value(1, tag("")),
    ))(i)
}

fn sign(i: &str) -> IResult<&str, &str> {
    alt((tag("+"), tag("-"), tag("")))(i)
}

#[derive(Copy, Clone)]
enum Exactness {
    Inexact,
    Exact,
    Unspecified,
}
use Exactness::*;

fn exactness(i: &str) -> IResult<&str, Exactness> {
    alt((
        value(Inexact, tag("#i")),
        value(Exact, tag("#e")),
        value(Unspecified, tag("")),
    ))(i)
}

fn radix<const R: u8>(i: &str) -> IResult<&str, ()> {
    match R {
        2 => value((), tag("#b"))(i),
        8 => value((), tag("#o"))(i),
        10 => value((), alt((tag("#d"), tag(""))))(i),
        16 => value((), tag("#x"))(i),
        _ => unreachable!("only radices 2, 8, 10 and 16 work"),
    }
}

fn digit<const R: u8>(i: &str) -> IResult<&str, u8> {
    for (x, &c) in [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
    ][0..(R as usize)]
        .into_iter()
        .enumerate()
    {
        if i.chars().next() == Some(c) {
            return Ok((i.split_at(1).1, x as u8));
        }
    }
    nom::combinator::fail(i)
}

fn datum(i: &str) -> IResult<&str, &str> {
    nom::combinator::fail(i)
}
