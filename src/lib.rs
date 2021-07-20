mod basex;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let mut client = crate::basex::connect("basex", 1984, "admin", "admin").unwrap();
        let info = client.create("lambada", Some("<None><Text></Text><Lala></Lala><Papa></Papa></None>")).unwrap();
        println!("{}", &info);
        let mut query = client.query("count(/None/*)").unwrap();
        let result = query.execute().unwrap();
        assert_eq!(result, "3");
        let _ = query.close().unwrap();
    }
}
