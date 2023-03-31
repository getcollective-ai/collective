use builder::Build;

#[derive(Default, Build)]
struct Request {
    url: String,
    path: Option<String>
}


#[test]
fn test_builder() {
    let request = Request::new()
        .url("tester")
        .path("abc/xyz");
}