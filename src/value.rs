use std::mem;

use combine::easy::Error;
use combine::error::StreamError;

use format::{Displayable, Formatter};
use position::Pos;
use tokenizer::Token;

/// Generic string value
///
/// It may consist of strings and variable references
///
/// Some string parts might originally be escaped or quoted. We get rid of
/// quotes when parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value {
    position: Pos,
    data: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Item {
    Literal(String),
    Variable(String),
}


impl Value {
    pub(crate) fn parse<'a>(position: Pos, tok: Token<'a>)
        -> Result<Value, Error<Token<'a>, Token<'a>>>
    {
        let data = if tok.value.starts_with('"') {
            Value::scan_quoted('"', tok.value)?
        } else if tok.value.starts_with("'") {
            Value::scan_quoted('\'', tok.value)?
        } else {
            Value::scan_raw(tok.value)?
        };
        Ok(Value { position, data })
    }

    fn scan_raw<'a>(value: &str)
        -> Result<Vec<Item>, Error<Token<'a>, Token<'a>>>
    {
        use self::Item::*;
        let mut buf = Vec::new();
        let mut chiter = value.char_indices().peekable();
        let mut prev_char = ' ';  // any having no special meaning
        let mut cur_slice = 0;
        // TODO(unquote) single and double quotes
        while let Some((idx, cur_char)) = chiter.next() {
            match cur_char {
                _ if prev_char == '\\' => {
                    prev_char = ' ';
                    continue;
                }
                '$' => {
                    let vstart = idx + 1;
                    if idx != cur_slice {
                        buf.push(Literal(value[cur_slice..idx].to_string()));
                    }
                    let fchar = chiter.next().map(|(_, c)| c)
                        .ok_or_else(|| Error::unexpected_message(
                            "bare $ in expression"))?;
                    match fchar {
                        '{' => {
                            unimplemented!();
                        }
                        'a'...'z' | 'A'...'Z' | '_' => {
                            while let Some(&(_, c)) = chiter.peek() {
                                match c {
                                    'a'...'z' | 'A'...'Z' | '_' | '0'...'9'
                                    => chiter.next(),
                                    _ => break,
                                };
                            }
                            let now = chiter.peek().map(|&(idx, _)| idx)
                                .unwrap_or(value.len());
                            buf.push(Variable(
                                value[vstart..now].to_string()));
                            cur_slice = now;
                        }
                        _ => {
                            return Err(Error::unexpected_message(
                                format!("variable name starts with \
                                    bad char {:?}", fchar)));
                        }
                    }
                }
                _ => {}
            }
            prev_char = cur_char;
        }
        if cur_slice != value.len() {
            buf.push(Literal(value[cur_slice..].to_string()));
        }
        Ok(buf)
    }

    fn scan_quoted<'a>(quote: char, value: &str)
        -> Result<Vec<Item>, Error<Token<'a>, Token<'a>>>
    {
        use self::Item::*;
        let mut buf = Vec::new();
        let mut chiter = value.char_indices().peekable();
        chiter.next(); // skip quote
        let mut prev_char = ' ';  // any having no special meaning
        let mut cur_slice = String::new();
        while let Some((idx, cur_char)) = chiter.next() {
            match cur_char {
                _ if prev_char == '\\' => {
                    cur_slice.push(cur_char);
                    continue;
                }
                '"' | '\'' if cur_char == quote => {
                    if cur_slice.len() > 0 {
                        buf.push(Literal(cur_slice));
                    }
                    if idx + 1 != value.len() {
                        // TODO(tailhook) figure out maybe this is actually a
                        // tokenizer error, or maybe make this cryptic message
                        // better
                        return Err(Error::unexpected_message(
                            "quote closes prematurely"));
                    }
                    return Ok(buf);
                }
                '$' => {
                    let vstart = idx + 1;
                    if cur_slice.len() > 0 {
                        buf.push(Literal(
                            mem::replace(&mut cur_slice, String::new())));
                    }
                    let fchar = chiter.next().map(|(_, c)| c)
                        .ok_or_else(|| Error::unexpected_message(
                            "bare $ in expression"))?;
                    match fchar {
                        '{' => {
                            unimplemented!();
                        }
                        'a'...'z' | 'A'...'Z' | '_' => {
                            while let Some(&(_, c)) = chiter.peek() {
                                match c {
                                    'a'...'z' | 'A'...'Z' | '_' | '0'...'9'
                                    => chiter.next(),
                                    _ => break,
                                };
                            }
                            let now = chiter.peek().map(|&(idx, _)| idx)
                                .ok_or_else(|| {
                                    Error::unexpected_message("unclosed quote")
                                })?;
                            buf.push(Variable(
                                value[vstart..now].to_string()));
                        }
                        _ => {
                            return Err(Error::unexpected_message(
                                format!("variable name starts with \
                                    bad char {:?}", fchar)));
                        }
                    }
                }
                _ => cur_slice.push(cur_char),
            }
            prev_char = cur_char;
        }
        return Err(Error::unexpected_message("unclosed quote"));
    }
}

impl Value {
    fn has_specials(&self) -> bool {
        use self::Item::*;
        for item in &self.data {
            match *item {
                Literal(ref x) => {
                    for c in x.chars() {
                        match c {
                            ' ' | ';' | '\r' | '\n' | '\t' => {
                                return true;
                            }
                            _ => {}
                        }
                    }
                }
                Variable(_) => {}
            }
        }
        return false;
    }
}

impl Displayable for Value {
    fn display(&self, f: &mut Formatter) {
        use self::Item::*;
        if self.has_specials() {
            f.write("\"");
            for item in &self.data {
                match *item {
                    // TODO(tailhook) escape special chars
                    Literal(ref v) => f.write(v),
                    Variable(ref v) => { f.write("$"); f.write(v); }
                }
            }
            f.write("\"");
        } else {
            for item in &self.data {
                match *item {
                    Literal(ref v) => f.write(v),
                    Variable(ref v) => { f.write("$"); f.write(v); }
                }
            }
        }
    }
}
