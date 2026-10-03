//! The authored stylesheet is self-contained. Asset-loading syntax is unsupported.
use cssparser::{ParseError, Parser, ParserInput, Token};

fn tokens<'i>(parser: &mut Parser<'i, '_>) -> Result<(), ParseError<'i, &'static str>> {
    while !parser.is_exhausted() {
        match parser.next()?.clone() {
            Token::UnquotedUrl(_) | Token::BadUrl(_) => {
                return Err(parser.new_custom_error("CSS asset URLs are unsupported"));
            }
            Token::AtKeyword(name) if name.eq_ignore_ascii_case("import") => {
                return Err(parser.new_custom_error("CSS imports are unsupported"));
            }
            Token::Function(name)
                if ["url", "image", "image-set", "-webkit-image-set", "src"]
                    .iter()
                    .any(|asset| name.eq_ignore_ascii_case(asset)) =>
            {
                return Err(parser.new_custom_error("CSS asset functions are unsupported"));
            }
            Token::Function(_)
            | Token::ParenthesisBlock
            | Token::SquareBracketBlock
            | Token::CurlyBracketBlock => {
                parser.parse_nested_block(tokens)?;
            }
            _ => {}
        }
    }
    Ok(())
}

pub fn check(bytes: &[u8]) -> Result<(), String> {
    let css = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
    let mut input = ParserInput::new(css);
    tokens(&mut Parser::new(&mut input)).map_err(|e| format!("stylesheet refused: {e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_text_and_comments_do_not_load_assets() {
        check(br#"/* url(missing.png) @import 'missing.css'; */
            @media (max-width: 600px) { p::after { content: "url(missing.png)"; color: rgb(1, 2, 3); } }
        "#).unwrap();
    }
}
