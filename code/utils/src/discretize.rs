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
    use super::string;

    #[test]
    fn test_discretize_simple() {
        let input = "Hello, world!";
        let res = string(input);
        assert_eq!(vec!["Hello, world!"], res);
    }

    #[test]
    fn test_discretize_paragraph() {
        let lorem = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Donec auctor, nisl \
                     eget ultricies lacinia, nisl nisl aliquet nisl, eget aliquet nunc";

        let base_len = lorem.chars().count();

        let lorem = std::iter::once(lorem).cycle();

        let take = 8000 / base_len;
        let lorem = lorem.take(take).collect::<Vec<_>>().join(" ");

        let res = string(&lorem);
        assert_eq!(res.len(), 4);
    }
}
