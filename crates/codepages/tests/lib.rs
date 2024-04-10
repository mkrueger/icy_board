#[cfg(test)]
mod test {
    use codepages::tables::get_utf8;
    #[test]
    fn test_get_from_cp437() {
        assert_eq!(get_utf8(&[226, 148, 140, 226, 148, 128]), "┌─");
    }

    #[test]
    fn test_get_from_uft8() {
        assert_eq!(
            get_utf8(&"\u{1F680}\u{1F642}".to_string().bytes().into_iter().collect::<Vec<u8>>()),
            "\u{1F680}\u{1F642}"
        );
    }

    #[test]
    fn test_get_from_ascii() {
        assert_eq!(get_utf8(&"Hello World".to_string().bytes().into_iter().collect::<Vec<u8>>()), "Hello World");
    }
}
