use std::collections::{HashMap, VecDeque};

use thiserror::Error;

#[derive(Clone, Copy)]
pub enum Interpolator {
    DoubleCurlyBrackets,
    DoubleBrackets,
}

pub fn interpolate_variables(
    input: &str,
    variables: &HashMap<String, String>,
    interpolator: Interpolator,
) -> Result<(String, String), SviError> {
    let mut result = String::new();
    let mut replace_for_readable = Vec::new();

    let (double_opener, single_opener, triple_closer, double_closer) = match interpolator {
        Interpolator::DoubleCurlyBrackets => ("{{", "{", "}}}", "}}"),
        Interpolator::DoubleBrackets => ("[[", "[", "]]]", "]]"),
    };

    let mut open_split = input.split(double_opener).collect::<VecDeque<_>>();
    let first = open_split.pop_front().ok_or(SviError::SplitEmpty)?;
    result.push_str(first);

    for i in 0..open_split.len() {
        if open_split[i].get(0..1).is_none() {
            return Err(SviError::UsedDoubleOpener {
                double_opener: double_opener.to_string(),
            });
        }
        if &open_split[i][0..1] == single_opener {
            result.push_str(single_opener);
            let split2 = open_split[i].split(triple_closer).collect::<Vec<_>>();
            for i in 0..split2.len() {
                result.push_str(split2[i]);
                if i == 0 && split2.len() > 1 {
                    result.push_str(double_closer);
                }
            }
        } else {
            let split2 = open_split[i].split(double_closer).collect::<Vec<_>>();
            if split2.len() <= 1 {
                return Err(SviError::NoClosingTags { index: i });
            }
            let variable = split2[0];
            let value = variables.get(variable).ok_or(SviError::NoValueFound {
                variable: variable.to_string(),
            })?;
            replace_for_readable.push((value.clone(), format!("<{variable}>")));
            result.push_str(value);
            result.push_str(&split2[1..].join(""));
        }
    }

    let mut readable = result.clone();

    for (to_replace, replacer) in replace_for_readable {
        readable = readable.replace(&to_replace, &replacer);
    }

    Ok((result, readable))
}

#[derive(Error, Debug)]
pub enum SviError {
    #[error("split is empty. this shouldn't happen")]
    SplitEmpty,
    #[error("found double opener '{double_opener}{double_opener}', formatting will fail")]
    UsedDoubleOpener { double_opener: String },
    #[error("no closing tags for variable at index {index}")]
    NoClosingTags { index: usize },
    #[error("did not find any value for variable {variable}")]
    NoValueFound { variable: String },
}
