mod basex;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        match crate::basex::connect("basex", 1984, "admin", "admin") {
            Ok(_c) => {},
            Err(e) => println!("{}", e)
        }
        assert_eq!(2 + 2, 4);
    }
}
