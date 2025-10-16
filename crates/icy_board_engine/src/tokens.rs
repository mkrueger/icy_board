pub fn tokenize(str: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut cur = String::new();
    if str.len() == 0 {
        return result;
    }

    for c in str.chars() {
        if c == ';' {
            result.push(cur.trim_ascii_end().to_string());
            cur.clear();
        } else if c == ' ' {
            if cur.len() == 0 {
                continue;
            }
            result.push(cur.trim_ascii_end().to_string());
            cur.clear();
        } else {
            cur.push(c);
        }
    }
    if cur.len() > 0 {
        result.push(cur.trim_ascii_end().to_string());
    }

    result
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_tokenize() {
        let input = "Hello world";
        let expected = vec!["Hello", "world"];
        assert_eq!(tokenize(input), expected);
    }

    #[test]
    fn test_tokenize_semicolon() {
        let input = "Hello;world";
        let expected = vec!["Hello", "world"];
        assert_eq!(tokenize(input), expected);
    }

    #[test]
    fn test_tokenize_empty_semicolon() {
        let input = "Hello;;world";
        let expected = vec!["Hello", "", "world"];
        assert_eq!(tokenize(input), expected);
    }

    #[test]
    fn test_tokenize_semicolon2() {
        let input = "Hello; world";
        let expected = vec!["Hello", "world"];
        assert_eq!(tokenize(input), expected);
    }

    #[test]
    fn test_tokenize_empty() {
        let input = "";
        assert!(!tokenize(input).is_empty());
    }
}
