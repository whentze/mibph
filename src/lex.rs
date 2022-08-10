use nom::{
    self,
    branch::{alt, permutation},
    bytes::complete::{is_not, tag, take_until},
    character::complete::{anychar, char, none_of, one_of, satisfy},
    combinator::{recognize, value},
    multi::{many0, many0_count, many1_count},
    sequence::{delimited, pair, preceded, separated_pair, tuple},
    AsChar, IResult,
};

#[derive(Debug, Clone)]
pub enum Token {
    Identifier,
    Boolean,
    Number,
    Todo,
    Character,
    String,
    Other
}

pub fn token(i: &str) -> IResult<&str, Token> {
    alt((
        value (
            Token::Identifier,
            identifier,
        ),
        value (
            Token::Boolean,
            boolean,
        ),
        value (
            Token::Number,
            number,
        ),
        value (
            Token::Character,
            character,
        ),
        value (
            Token::String,
            string,
        ),
        value (
            Token::Other,
            alt((
                tag("("),
                tag(")"),
                tag("#("),
                tag("#u8("),
                tag("'"),
                tag("`"),
                tag(","),
                tag(",@"),
                tag("."),
            ))
        )
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
        //preceded(pair(tag("#;"), intertoken_space), datum),
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
    alt((initial, digit::<10>, special_subsequent))(i)
}

fn explicit_sign(i: &str) -> IResult<&str, char> {
    one_of(&['+', '-'] as &[char])(i)
}

fn special_subsequent(i: &str) -> IResult<&str, char> {
    alt((explicit_sign, char('.'), char('@')))(i)
}

fn inline_hex_escape(i: &str) -> IResult<&str, &str> {
    preceded(tag(r"\x"), hex_scalar_value)(i)
}

fn hex_scalar_value(i: &str) -> IResult<&str, &str> {
    recognize(many1_count(digit::<16>))(i)
}

fn mnemonic_escape(i: &str) -> IResult<&str, &str> {
    alt((tag(r"\a"), tag(r"\b"), tag(r"\t"), tag(r"\n"), tag(r"\r")))(i)
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

fn symbol_element(i: &str) -> IResult<&str, &str> {
    alt((
        recognize(none_of(&['|', '\\'] as &[char])),
        inline_hex_escape,
        mnemonic_escape,
        tag(r"\|"),
    ))(i)
}

fn boolean(i: &str) -> IResult<&str, &str> {
    alt((tag("#true"), tag("#false"), tag("#t"), tag("#f"), ))(i)
}

fn character(i: &str) -> IResult<&str, &str> {
    alt((
        preceded(tag(r"#\"), recognize(anychar)),
        preceded(tag(r"#\"), character_name),
        preceded(tag(r"#\x"), hex_scalar_value),
    ))(i)
}

fn character_name(i: &str) -> IResult<&str, &str> {
    alt((
        tag("alarm"),
        tag("backspace"),
        tag("delete"),
        tag("escape"),
        tag("newline"),
        tag("null"),
        tag("return"),
        tag("space"),
        tag("tab"),
    ))(i)
}

fn string(i: &str) -> IResult<&str, &str> {
    delimited(tag("\""), recognize(many0_count(string_element)), tag("\""))(i)
}

fn string_element(i: &str) -> IResult<&str, &str> {
    alt((
        recognize(none_of(&['"', '\\'] as &[char])),
        mnemonic_escape,
        tag(r#"\""#),
        tag(r#"\\"#),
        recognize(tuple((
            tag(r"\"),
            many0_count(intraline_whitespace),
            line_ending,
            many0_count(intraline_whitespace),
        ))),
        inline_hex_escape,
    ))(i)
}

fn bytevector(i: &str) -> IResult<&str, &str> {
    delimited(tag("#u8("), recognize(many0_count(byte)), tag(")"))(dbg!(i))
}

fn byte(i: &str) -> IResult<&str, &str> {
    // TODO: restrict
    number(i)
}

fn number(i: &str) -> IResult<&str, &str> {
    alt((num::<2>, num::<8>, num::<10>, num::<16>))(i)
}

fn num<const R: u8>(i: &str) -> IResult<&str, &str> {
    recognize(pair(prefix::<R>, complex::<R>))(i)
}

fn complex<const R: u8>(i: &str) -> IResult<&str, &str> {
    // TODO: all the other cases
    real::<R>(i)
}

fn real<const R: u8>(i: &str) -> IResult<&str, &str> {
    alt((recognize(pair(sign, ureal::<R>)), infnan))(i)
}

fn ureal<const R: u8>(i: &str) -> IResult<&str, &str> {
    alt((
        recognize(separated_pair(uinteger::<R>, tag("/"), uinteger::<R>)),
        decimal::<R>,
        uinteger::<R>,
    ))(i)
}

fn decimal<const R: u8>(i: &str) -> IResult<&str, &str> {
    match R {
        10 => alt((
            recognize(tuple((
                many1_count(digit::<10>),
                tag("."),
                many0_count(digit::<10>),
                suffix,
            ))),
            recognize(preceded(tag("."), pair(many1_count(digit::<10>), suffix))),
            recognize(pair(uinteger::<10>, suffix)),
        ))(i),
        _ => nom::combinator::fail(i),
    }
}

fn uinteger<const R: u8>(i: &str) -> IResult<&str, &str> {
    recognize(many1_count(digit::<R>))(i)
}

fn prefix<const R: u8>(i: &str) -> IResult<&str, &str> {
    alt((recognize(permutation((radix::<R>, exactness))),))(i)
}

fn infnan(i: &str) -> IResult<&str, &str> {
    alt((tag("+inf.0"), tag("-inf.0"), tag("+nan.0"), tag("-nan.0")))(i)
}

fn suffix(i: &str) -> IResult<&str, &str> {
    alt((
        recognize(tuple((tag("e"), sign, many1_count(digit::<10>)))),
        tag(""),
    ))(i)
}

fn sign(i: &str) -> IResult<&str, &str> {
    alt((tag("+"), tag("-"), tag("")))(i)
}

fn exactness(i: &str) -> IResult<&str, &str> {
    alt((tag("#i"), tag("#e"), tag("")))(i)
}

fn radix<const R: u8>(i: &str) -> IResult<&str, &str> {
    match R {
        2 => tag("#b")(i),
        8 => tag("#o")(i),
        10 => alt((tag("#d"), tag("")))(i),
        16 => tag("#x")(i),
        _ => unreachable!("only radices 2, 8, 10 and 16 work"),
    }
}

fn digit<const R: u8>(i: &str) -> IResult<&str, char> {
    match R {
        2 => one_of(&['0', '1'] as &[char])(i),
        8 => satisfy(AsChar::is_oct_digit)(i),
        10 => satisfy(AsChar::is_dec_digit)(i),
        16 => satisfy(AsChar::is_hex_digit)(i),
        _ => unreachable!("only radices 2, 8, 10 and 16 work"),
    }
}
