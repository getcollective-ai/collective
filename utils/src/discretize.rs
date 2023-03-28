use once_cell::sync::Lazy;
use regex::Regex;

pub fn string(input: &str) -> Vec<&str> {
    let mut result = Vec::new();

    static WORD: Lazy<Regex> = Lazy::new(|| Regex::new(r".{1,4000}\s?").unwrap());

    WORD.find_iter(input).for_each(|m| {
        result.push(m.as_str());
    });

    if let Some((idx, _)) = input.char_indices().nth(2000) {
        let input = &input[idx..];
        WORD.find_iter(input).for_each(|m| {
            result.push(m.as_str());
        });
    }

    result
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_discretize_markdown() {
        let input = "Hello, world!";
        let res = super::string(input);
        assert_eq!(vec!["Hello, world!"], res);
    }
}
