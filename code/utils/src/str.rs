mod sealed {
    pub trait Sealed {}
}

pub trait StringExt: sealed::Sealed {
    /// trim the string in place
    fn trim_end_in_place(&mut self);
}

impl sealed::Sealed for String {}

impl StringExt for String {
    fn trim_end_in_place(&mut self) {
        self.truncate(self.trim_end().len());
    }
}

#[cfg(test)]
mod tests {
    use crate::str::StringExt;

    #[test]
    fn test_trim_end_in_place() {
        let mut s = "hello there".to_string();
        s.trim_end_in_place();
        assert_eq!(s, "hello there");

        let mut s = "".to_string();
        s.trim_end_in_place();
        assert_eq!(s, "");

        let mut s = " ".to_string();
        s.trim_end_in_place();
        assert_eq!(s, "");

        let mut s = "hello there ".to_string();
        s.trim_end_in_place();
        assert_eq!(s, "hello there");

        let mut s = " hello there ".to_string();
        s.trim_end_in_place();
        assert_eq!(s, " hello there");
    }
}
